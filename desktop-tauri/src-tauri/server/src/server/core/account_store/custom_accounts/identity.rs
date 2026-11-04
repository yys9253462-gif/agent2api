use rusqlite::Connection;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::CUSTOM_ACCOUNT_ID_PREFIX;
use crate::server::core::account_store::{sql, state::StoredAccount};

pub(super) fn existing(
    conn: &Connection,
    provider_id: &str,
    api_key: &str,
) -> rusqlite::Result<Option<StoredAccount>> {
    if api_key.is_empty() {
        return Ok(None);
    }
    let mut statement =
        conn.prepare("SELECT id FROM accounts WHERE provider = ?1 ORDER BY rowid")?;
    let ids = statement
        .query_map([provider_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    // 按实际凭证兼容旧 ID；写入前逐条严格加载，不把解密失败当作账号不存在。
    for id in ids {
        if let Some(record) = sql::load_by_id(conn, &id)? {
            if record.fields().get("apiKey").and_then(Value::as_str) == Some(api_key) {
                return Ok(Some(record));
            }
        }
    }
    Ok(None)
}

pub(super) fn new_id(provider_id: &str, api_key: &str) -> Result<String, String> {
    let suffix = if api_key.is_empty() {
        let mut bytes = [0u8; 6];
        getrandom::getrandom(&mut bytes)
            .map_err(|_| "无法生成安全的随机账号 id（系统随机源不可用），请重试".to_string())?;
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    } else {
        let mut hash = Sha256::new();
        hash.update(provider_id.as_bytes());
        hash.update([0]);
        hash.update(api_key.as_bytes());
        format!("{:x}", hash.finalize())
            .chars()
            .take(12)
            .collect::<String>()
    };
    Ok(format!("{CUSTOM_ACCOUNT_ID_PREFIX}{suffix}"))
}
