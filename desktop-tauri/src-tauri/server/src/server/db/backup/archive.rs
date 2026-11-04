use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;

use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD, Engine};
use flate2::{bufread::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use zeroize::Zeroizing;

use super::keys::{self, BackupKeys, KeyArtifact};
use super::stream::{DecryptReader, EncryptWriter, CHUNK_BYTES};
use super::{dump_sql_into, snapshot_to_memory, table_row_counts, temp, Db, LOG_TABLES};

pub const BACKUP_EXTENSION: &str = "a2ab";
pub const MAX_BACKUP_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAGIC: &[u8; 16] = b"Agent2API-BKUP\n\0";
const FORMAT_VERSION: u32 = 1;
const MAX_METADATA: usize = 64 * 1024;
const MAX_SQL_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MEMORY_KIB: u32 = 64 * 1024;
const PASSES: u32 = 3;
const LANES: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Header {
    version: u32,
    kdf: String,
    memory_kib: u32,
    passes: u32,
    lanes: u32,
    salt: String,
    nonce: String,
    chunk_bytes: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    format: String,
    version: u32,
    app_version: String,
    schema_version: i64,
    created_at: i64,
    include_logs: bool,
    keys: BTreeMap<String, KeyArtifact>,
}

pub(super) struct ImportedBackup {
    pub sql: std::fs::File,
    pub keys: Option<BackupKeys>,
    pub schema_version: Option<i64>,
}

pub(super) fn validate_password(password: &str, exporting: bool) -> Result<(), String> {
    if password.len() > 1024 || (exporting && password.chars().count() < 12) {
        return Err("备份密码需要至少 12 位，且不能超过 1024 字节".into());
    }
    if password.is_empty() {
        return Err("请输入备份密码；旧 SQL 备份可以留空".into());
    }
    Ok(())
}

fn derive(password: &str, header: &Header) -> Result<Zeroizing<[u8; 32]>, String> {
    if header.version != FORMAT_VERSION
        || header.kdf != "argon2id"
        || header.chunk_bytes != CHUNK_BYTES
    {
        return Err("备份格式或加密算法不受当前版本支持，请升级应用".into());
    }
    if !(19 * 1024..=256 * 1024).contains(&header.memory_kib)
        || !(2..=6).contains(&header.passes)
        || !(1..=4).contains(&header.lanes)
    {
        return Err("备份密码派生参数异常".into());
    }
    let salt = STANDARD.decode(&header.salt).map_err(|_| "备份盐值损坏")?;
    if salt.len() != 32 {
        return Err("备份盐值长度异常".into());
    }
    let params = Params::new(header.memory_kib, header.passes, header.lanes, Some(32))
        .map_err(|_| "备份密码派生参数不受支持")?;
    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), &salt, key.as_mut())
        .map_err(|_| "备份密码派生失败")?;
    Ok(key)
}

pub(super) fn export(
    db: &Db,
    writer: &mut impl Write,
    include_logs: bool,
    password: &str,
    config_dir: &Path,
) -> Result<Value, String> {
    validate_password(password, true)?;
    let snapshot = snapshot_to_memory(db)?;
    let keys = BackupKeys::collect()?;
    keys::validate(&snapshot, &keys)?;
    let skip = if include_logs { &[][..] } else { LOG_TABLES };
    let tables = table_row_counts(&snapshot, skip);
    let schema_version = snapshot
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| format!("读取备份版本失败: {error}"))?;
    let manifest = Manifest {
        format: "agent2api-complete-backup".into(),
        version: FORMAT_VERSION,
        app_version: env!("CARGO_PKG_VERSION").into(),
        schema_version,
        created_at: crate::server::logging::now_ms(),
        include_logs,
        keys: keys.artifacts(),
    };
    let mut salt = [0u8; 32];
    let mut nonce = [0u8; 7];
    getrandom::getrandom(&mut salt).map_err(|_| "生成备份随机盐失败")?;
    getrandom::getrandom(&mut nonce).map_err(|_| "生成备份随机数失败")?;
    let header = Header {
        version: FORMAT_VERSION,
        kdf: "argon2id".into(),
        memory_kib: MEMORY_KIB,
        passes: PASSES,
        lanes: LANES,
        salt: STANDARD.encode(salt),
        nonce: STANDARD.encode(nonce),
        chunk_bytes: CHUNK_BYTES,
    };
    let header_json = serde_json::to_vec(&header).map_err(|_| "生成备份头部失败")?;
    let mut authenticated_header = MAGIC.to_vec();
    authenticated_header.extend_from_slice(&(header_json.len() as u32).to_be_bytes());
    authenticated_header.extend_from_slice(&header_json);
    let key = derive(password, &header)?;
    let mut counted = LimitedWriter { writer, written: 0 };
    counted
        .write_all(&authenticated_header)
        .map_err(write_error)?;
    let encrypted = EncryptWriter::new(&mut counted, &key, &nonce, authenticated_header)
        .map_err(write_error)?;
    let mut compressed = GzEncoder::new(encrypted, Compression::default());
    let manifest_bytes =
        Zeroizing::new(serde_json::to_vec(&manifest).map_err(|_| "生成备份清单失败")?);
    compressed
        .write_all(&(manifest_bytes.len() as u32).to_be_bytes())
        .map_err(write_error)?;
    compressed.write_all(&manifest_bytes).map_err(write_error)?;
    let mut sql_file = temp::anonymous_file(config_dir).map_err(write_error)?;
    dump_sql_into(&snapshot, skip, &mut sql_file)?;
    let sql_bytes = sql_file.stream_position().map_err(write_error)?;
    if sql_bytes > MAX_SQL_BYTES {
        return Err("备份 SQL 超过 8GB 上限，请改用精简导出".into());
    }
    sql_file.seek(SeekFrom::Start(0)).map_err(write_error)?;
    compressed
        .write_all(&sql_bytes.to_be_bytes())
        .map_err(write_error)?;
    io::copy(&mut sql_file, &mut compressed).map_err(write_error)?;
    compressed
        .finish()
        .map_err(write_error)?
        .finish()
        .map_err(write_error)?;
    Ok(json!({
        "bytes": counted.written,
        "tables": tables,
        "encrypted": true,
        "includedKeys": keys.names(),
        "format": "agent2api-complete-backup",
    }))
}

pub(super) fn read(
    source: &Path,
    password: &str,
    config_dir: &Path,
) -> Result<ImportedBackup, String> {
    if password.len() > 1024 {
        return Err("备份密码不能超过 1024 字节".into());
    }
    let file = std::fs::File::open(source).map_err(|error| format!("读取备份文件失败: {error}"))?;
    let file_size = file
        .metadata()
        .map_err(|error| format!("读取备份大小失败: {error}"))?
        .len();
    if file_size > MAX_BACKUP_BYTES {
        return Err("备份文件超过 2GB 上限".into());
    }
    let mut reader = BufReader::new(file);
    if !reader.fill_buf().map_err(read_error)?.starts_with(MAGIC) {
        return Ok(ImportedBackup {
            sql: reader.into_inner(),
            keys: None,
            schema_version: None,
        });
    }
    validate_password(password, false)?;
    let mut authenticated_header = vec![0u8; MAGIC.len() + 4];
    reader
        .read_exact(&mut authenticated_header)
        .map_err(read_error)?;
    let header_size = u32::from_be_bytes(
        authenticated_header[MAGIC.len()..]
            .try_into()
            .map_err(|_| "备份头部损坏")?,
    ) as usize;
    if header_size == 0 || header_size > MAX_METADATA {
        return Err("备份头部长度异常".into());
    }
    authenticated_header.resize(MAGIC.len() + 4 + header_size, 0);
    reader
        .read_exact(&mut authenticated_header[MAGIC.len() + 4..])
        .map_err(read_error)?;
    let header: Header = serde_json::from_slice(&authenticated_header[MAGIC.len() + 4..])
        .map_err(|_| "备份头部格式损坏")?;
    let nonce: [u8; 7] = STANDARD
        .decode(&header.nonce)
        .map_err(|_| "备份随机数损坏")?
        .try_into()
        .map_err(|_| "备份随机数长度异常")?;
    let key = derive(password, &header)?;
    let decrypted =
        DecryptReader::new(reader, &key, &nonce, authenticated_header).map_err(read_error)?;
    let mut compressed = GzDecoder::new(BufReader::new(decrypted));
    let mut size = [0u8; 4];
    compressed.read_exact(&mut size).map_err(read_error)?;
    let manifest_size = u32::from_be_bytes(size) as usize;
    if manifest_size == 0 || manifest_size > MAX_METADATA {
        return Err("备份清单长度异常".into());
    }
    let mut manifest_bytes = Zeroizing::new(vec![0u8; manifest_size]);
    compressed
        .read_exact(&mut manifest_bytes)
        .map_err(read_error)?;
    let manifest: Manifest =
        serde_json::from_slice(&manifest_bytes).map_err(|_| "备份清单格式损坏")?;
    if manifest.format != "agent2api-complete-backup" || manifest.version != FORMAT_VERSION {
        return Err("备份清单版本不受当前应用支持".into());
    }
    let mut sql_size = [0u8; 8];
    compressed.read_exact(&mut sql_size).map_err(read_error)?;
    let sql_size = u64::from_be_bytes(sql_size);
    if sql_size == 0 || sql_size > MAX_SQL_BYTES {
        return Err("备份解压后 SQL 超过上限或内容为空".into());
    }
    let mut sql = temp::anonymous_file(config_dir).map_err(read_error)?;
    let written =
        io::copy(&mut compressed.by_ref().take(sql_size), &mut sql).map_err(read_error)?;
    if written != sql_size {
        return Err("备份 SQL 不完整".into());
    }
    sql.seek(SeekFrom::Start(0)).map_err(read_error)?;
    let mut trailing = [0u8; 1];
    if compressed.read(&mut trailing).map_err(read_error)? != 0 {
        return Err("备份包含多余的解压数据".into());
    }
    let mut buffered = compressed.into_inner();
    if !buffered.fill_buf().map_err(read_error)?.is_empty() {
        return Err("备份压缩数据存在多余内容".into());
    }
    buffered.into_inner().finish().map_err(read_error)?;
    let keys = BackupKeys::from_artifacts(manifest.keys)?;
    Ok(ImportedBackup {
        sql,
        keys: Some(keys),
        schema_version: Some(manifest.schema_version),
    })
}

struct LimitedWriter<'a, W: Write> {
    writer: &'a mut W,
    written: u64,
}

impl<W: Write> Write for LimitedWriter<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.written.saturating_add(bytes.len() as u64) > MAX_BACKUP_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "备份文件超过 2GB 上限，请改用精简导出",
            ));
        }
        let count = self.writer.write(bytes)?;
        self.written += count as u64;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

fn write_error(error: io::Error) -> String {
    format!("写入加密备份失败: {error}")
}

fn read_error(error: io::Error) -> String {
    format!("读取备份失败: {error}")
}
