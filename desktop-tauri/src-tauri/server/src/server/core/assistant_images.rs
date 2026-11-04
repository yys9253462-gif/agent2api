//! 助手图片附件独立保存，消息历史只存 ID；删除会话时由外键一起删除图片。

use std::sync::OnceLock;

use rusqlite::{params, OptionalExtension};

use crate::server::db::Db;
use crate::server::errors::GatewayError;
use crate::server::logging;

static DB: OnceLock<Option<Db>> = OnceLock::new();
const MAX_STORAGE_BYTES: i64 = 512 * 1024 * 1024;

pub fn install(db: Option<Db>) {
    let _ = DB.set(db);
}

fn database() -> Result<&'static Db, GatewayError> {
    DB.get()
        .and_then(Option::as_ref)
        .ok_or_else(|| GatewayError::with_status(503, "助手图片存储不可用"))
}

pub fn available() -> Result<(), GatewayError> {
    let used = database()?
        .with(|conn| {
            conn.query_row(
                "SELECT COALESCE(SUM(length(bytes)), 0) FROM assistant_images",
                [],
                |row| row.get::<_, i64>(0),
            )
        })
        .ok_or_else(|| GatewayError::with_status(503, "助手图片数据库不可用"))?
        .map_err(|_| GatewayError::with_status(503, "助手图片存储无法读取"))?;
    if used >= MAX_STORAGE_BYTES {
        return Err(GatewayError::with_status(
            507,
            "助手图片存储已满，请删除不再需要的历史会话",
        ));
    }
    Ok(())
}

pub fn store(session: &str, images: Vec<(Vec<u8>, String)>) -> Result<Vec<String>, GatewayError> {
    let mut entries = Vec::with_capacity(images.len());
    for (bytes, mime) in images {
        let mut random = [0u8; 32];
        getrandom::getrandom(&mut random).map_err(|_| GatewayError::new("无法生成助手图片 ID"))?;
        let id: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        entries.push((id, bytes, mime));
    }
    let ids = entries.iter().map(|(id, _, _)| id.clone()).collect();
    database()?.with_mut(|conn| -> Result<(), GatewayError> {
        let tx = conn.transaction().map_err(|_| GatewayError::new("无法保存助手图片"))?;
        let used: i64 = tx.query_row("SELECT COALESCE(SUM(length(bytes)), 0) FROM assistant_images", [], |row| row.get(0))
            .map_err(|_| GatewayError::new("无法读取助手图片存储用量"))?;
        let added: i64 = entries.iter().map(|(_, bytes, _)| bytes.len() as i64).sum();
        if used.saturating_add(added) > MAX_STORAGE_BYTES {
            return Err(GatewayError::with_status(507, "助手图片存储已满，图片已生成但未保存，请删除不再需要的历史会话"));
        }
        for (id, bytes, mime) in entries {
            tx.execute("INSERT INTO assistant_images (id, session_id, mime, bytes, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, session, mime, bytes, logging::now_ms()])
                .map_err(|_| GatewayError::new("助手图片保存失败，图片已生成，请勿自动重发"))?;
        }
        tx.commit().map_err(|_| GatewayError::new("助手图片保存失败，图片已生成，请勿自动重发"))
    }).ok_or_else(|| GatewayError::with_status(503, "助手图片数据库不可用"))??;
    Ok(ids)
}

pub fn read(id: &str, session: Option<&str>) -> Option<(Vec<u8>, String)> {
    if id.len() != 64 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    database().ok()?.with(|conn| conn.query_row(
        "SELECT bytes, mime FROM assistant_images WHERE id = ?1 AND (?2 IS NULL OR session_id = ?2)",
        params![id, session], |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional().ok().flatten()).flatten()
}
