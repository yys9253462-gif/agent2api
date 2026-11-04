use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

use rusqlite::hooks::{AuthContext, Authorization};
use rusqlite::Connection;

use super::{import_authorizer, HEADER_SCAN_LINES};

const MAX_STATEMENT_BYTES: usize = 1024 * 1024 * 1024;
const CHUNK_BYTES: usize = 64 * 1024;

pub(super) fn header(file: &mut std::fs::File) -> Result<String, String> {
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("定位备份文件失败: {error}"))?;
    let mut reader = BufReader::new(file.by_ref().take(16 * 1024));
    let mut header = String::new();
    for _ in 0..HEADER_SCAN_LINES {
        if reader
            .read_line(&mut header)
            .map_err(|error| format!("读取备份头部失败: {error}"))?
            == 0
        {
            break;
        }
    }
    drop(reader);
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("定位备份文件失败: {error}"))?;
    Ok(header)
}

pub(super) fn apply(conn: &Connection, file: &mut std::fs::File) -> Result<(), String> {
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("定位备份 SQL 失败: {error}"))?;
    conn.authorizer(Some(import_authorizer))
        .map_err(|error| format!("设置备份 SQL 校验失败: {error}"))?;
    let result = execute(conn, file);
    let reset = conn.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
    result?;
    reset.map_err(|error| format!("清除备份 SQL 校验失败: {error}"))
}

fn execute(conn: &Connection, file: &mut std::fs::File) -> Result<(), String> {
    let mut reader = BufReader::with_capacity(CHUNK_BYTES, file);
    if reader
        .fill_buf()
        .map_err(|error| format!("读取备份 SQL 失败: {error}"))?
        .starts_with(&[0xef, 0xbb, 0xbf])
    {
        reader.consume(3);
    }
    let mut pending = Vec::new();
    let mut chunk = [0u8; CHUNK_BYTES];
    loop {
        let count = reader
            .read(&mut chunk)
            .map_err(|error| format!("读取备份 SQL 失败: {error}"))?;
        if count == 0 {
            break;
        }
        for part in chunk[..count].split_inclusive(|byte| *byte == b';') {
            if pending.len().saturating_add(part.len()) > MAX_STATEMENT_BYTES {
                return Err("备份中单条 SQL 超过 1GB 上限".into());
            }
            if part.contains(&0) {
                return Err("备份 SQL 含无效的零字节".into());
            }
            pending.extend_from_slice(part);
            if part.last() != Some(&b';') {
                continue;
            }
            // SQLite 自己识别字符串与触发器中的分号，只有完整语句才交给执行器。
            pending.push(0);
            let complete = unsafe { rusqlite::ffi::sqlite3_complete(pending.as_ptr().cast()) };
            pending.pop();
            if complete == rusqlite::ffi::SQLITE_NOMEM {
                return Err("检查备份 SQL 时内存不足".into());
            }
            if complete == 1 {
                apply_pending(conn, &pending)?;
                pending.clear();
            }
        }
    }
    if !pending.is_empty() {
        apply_pending(conn, &pending)?;
    }
    Ok(())
}

fn apply_pending(conn: &Connection, bytes: &[u8]) -> Result<(), String> {
    let sql = std::str::from_utf8(bytes).map_err(|_| "备份 SQL 不是合法 UTF-8 文本")?;
    if sql.contains('\0') {
        return Err("备份 SQL 含无效的零字节".into());
    }
    conn.execute_batch(sql)
        .map_err(|error| format!("执行备份 SQL 失败: {error}"))
}
