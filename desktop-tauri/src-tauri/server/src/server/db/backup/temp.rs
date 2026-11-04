use std::io;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use tempfile::{Builder, NamedTempFile, TempDir};

pub(super) fn private_directory(config_dir: &Path) -> Result<TempDir, String> {
    let mut builder = Builder::new();
    builder.prefix("agent2api-restore-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    match builder.tempdir() {
        Ok(directory) => Ok(directory),
        Err(system_error) => {
            std::fs::create_dir_all(config_dir)
                .map_err(|error| format!("创建备用暂存目录失败: {error}"))?;
            builder.tempdir_in(config_dir).map_err(|error| {
                format!("创建暂存目录失败（系统临时目录: {system_error}；配置目录: {error}）")
            })
        }
    }
}

pub fn transfer_file(config_dir: &Path) -> Result<NamedTempFile, String> {
    let mut builder = Builder::new();
    builder.prefix("agent2api-transfer-");
    builder.suffix(".a2ab");
    match builder.tempfile() {
        Ok(file) => Ok(file),
        Err(system_error) => {
            std::fs::create_dir_all(config_dir)
                .map_err(|error| format!("创建备用传输目录失败: {error}"))?;
            builder.tempfile_in(config_dir).map_err(|error| {
                format!("创建传输文件失败（系统临时目录: {system_error}；配置目录: {error}）")
            })
        }
    }
}

pub(super) fn anonymous_file(config_dir: &Path) -> io::Result<std::fs::File> {
    tempfile::tempfile().or_else(|_| {
        std::fs::create_dir_all(config_dir)?;
        tempfile::tempfile_in(config_dir)
    })
}

pub(super) struct TempRestoreDb {
    // 连接必须先于目录释放，Windows 才能删除仍有 WAL/SHM 的暂存库。
    pub conn: Connection,
    _directory: TempDir,
}

impl TempRestoreDb {
    pub fn new(config_dir: &Path) -> Result<Self, String> {
        let directory = private_directory(config_dir)?;
        let path: PathBuf = directory.path().join("restore.db");
        let conn = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )
        .map_err(|error| format!("创建暂存库失败（{}）: {error}", path.display()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")
            .map_err(|error| format!("设置暂存库失败: {error}"))?;
        Ok(Self {
            conn,
            _directory: directory,
        })
    }
}
