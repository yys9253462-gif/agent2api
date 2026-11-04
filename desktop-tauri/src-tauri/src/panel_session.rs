//! 桌面壳的本机管理会话，与网页管理员登录和模型 Key 相互独立。
//!
//! 凭证只在本进程内保存；网关启动后通过 Rust 调用签发，不对 HTTP 暴露入口。
//! 发送管理请求前校验服务端的会话状态，缺失、过期或被撤销时自动重建。

use std::sync::Mutex;

use serde_json::Value;

use crate::server::access::{self, local_session::Session};

static SESSION: Mutex<Option<Session>> = Mutex::new(None);
const RENEW_AHEAD_MS: i64 = 2 * 60 * 1000;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn ensure_session() -> Result<Session, String> {
    let mut current = SESSION.lock().map_err(|_| "本机管理会话状态不可用".to_string())?;
    if crate::startup::phase() == "stopped" {
        return Err("本地网关正在退出，无法建立管理会话".to_string());
    }
    if let Some(session) = current.as_ref() {
        if session.expires_at > now_ms().saturating_add(RENEW_AHEAD_MS)
            && access::local_session::valid(&session.access_token)
        {
            return Ok(session.clone());
        }
    }
    // 并发请求共用这把锁，只有一个调用负责重新签发。
    let issued = access::local_session::issue()?;
    *current = Some(issued.clone());
    Ok(issued)
}

pub fn clear() {
    if let Ok(mut current) = SESSION.lock() {
        if let Some(session) = current.take() {
            access::local_session::revoke(&session.access_token);
        }
    }
}

pub fn ensure_local_access() -> Result<(), String> {
    ensure_session().map(|_| ())
}

pub fn attach(builder: reqwest::RequestBuilder) -> Result<reqwest::RequestBuilder, String> {
    let session = ensure_session()?;
    Ok(builder.header("x-panel-token", session.access_token))
}

pub fn credential_headers() -> Result<serde_json::Map<String, Value>, String> {
    let session = ensure_session()?;
    let mut headers = serde_json::Map::new();
    headers.insert("x-panel-token".to_string(), Value::String(session.access_token));
    Ok(headers)
}

/// 只在明确的管理鉴权失败后调用；不轮换模型 Key 或网页用户的会话。
pub fn recover_rejected(headers: &reqwest::header::HeaderMap) -> Result<bool, String> {
    let rejected = headers.get("x-panel-token").and_then(|value| value.to_str().ok());
    let Some(rejected) = rejected else { return Ok(false) };
    let mut current = SESSION.lock().map_err(|_| "本机管理会话状态不可用".to_string())?;
    if crate::startup::phase() == "stopped" {
        return Err("本地网关正在退出，无法恢复管理会话".to_string());
    }
    if current.as_ref().is_some_and(|session| session.access_token != rejected) {
        return Ok(true);
    }
    if access::local_session::valid(rejected) {
        return Ok(false);
    }
    *current = Some(access::local_session::issue()?);
    Ok(true)
}
