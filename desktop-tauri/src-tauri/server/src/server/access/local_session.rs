//! 本机壳的短效管理会话：只在进程内签发与保存，不提供 HTTP 签发入口。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::{now_ms, sha256_hex, ACCESS_TTL};

static TOKENS: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();

fn tokens() -> &'static Mutex<HashMap<String, i64>> {
    TOKENS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Clone)]
pub struct Session {
    pub access_token: String,
    pub expires_at: i64,
}

pub fn issue() -> Result<Session, String> {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).map_err(|_| "无法生成本机管理会话".to_string())?;
    let access_token: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    let now = now_ms();
    let expires_at = now.saturating_add(ACCESS_TTL.as_millis() as i64);
    let mut table = tokens().lock().map_err(|_| "本机管理会话状态不可用".to_string())?;
    table.retain(|_, expiry| *expiry > now);
    table.insert(sha256_hex(&access_token), expires_at);
    Ok(Session { access_token, expires_at })
}

pub fn valid(token: &str) -> bool {
    valid_any(std::iter::once(token))
}

pub(super) fn valid_any<'a>(candidates: impl IntoIterator<Item = &'a str>) -> bool {
    let Ok(mut table) = tokens().lock() else { return false };
    let now = now_ms();
    table.retain(|_, expiry| *expiry > now);
    candidates.into_iter().any(|token| !token.is_empty() && table.contains_key(&sha256_hex(token)))
}

pub fn revoke(token: &str) {
    let Ok(mut table) = tokens().lock() else { return };
    table.remove(&sha256_hex(token));
}

pub(super) fn revoke_any(candidates: &[String]) {
    let Ok(mut table) = tokens().lock() else { return };
    for token in candidates {
        table.remove(&sha256_hex(token));
    }
}
