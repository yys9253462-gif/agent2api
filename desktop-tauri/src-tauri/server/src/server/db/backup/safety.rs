use std::path::{Path, PathBuf};

use chrono::Local;
use rusqlite::backup::Backup;
use rusqlite::Connection;

use super::complete_backup;

pub(super) fn create(main: &mut Connection, config_dir: &Path) -> Result<PathBuf, String> {
    let backup_dir = config_dir.join("backups");
    std::fs::create_dir_all(&backup_dir).map_err(|error| format!("创建备份目录失败: {error}"))?;
    let mut builder = tempfile::Builder::new();
    builder.prefix("pre-restore-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    let directory = builder
        .tempdir_in(&backup_dir)
        .map_err(|error| format!("创建安全备份目录失败: {error}"))?;
    let target = directory.path().join("database.db");
    let mut dest =
        Connection::open(&target).map_err(|error| format!("创建安全备份失败: {error}"))?;
    {
        let backup =
            Backup::new(main, &mut dest).map_err(|error| format!("初始化安全备份失败: {error}"))?;
        complete_backup(&backup, "创建安全备份")?;
    }
    dest.close()
        .map_err(|(_, error)| format!("关闭安全备份失败: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("设置安全备份权限失败: {error}"))?;
    }
    let random = directory
        .path()
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let name = format!(
        "pre-restore-{}-{}.db",
        Local::now().format("%Y%m%d_%H%M%S"),
        random.trim_start_matches("pre-restore-")
    );
    let saved = backup_dir.join(name);
    std::fs::rename(&target, &saved).map_err(|error| format!("保存安全备份失败: {error}"))?;
    Ok(saved)
}
