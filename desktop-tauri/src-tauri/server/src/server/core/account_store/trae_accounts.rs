//! Trae（字节跳动 AI IDE）的**存量记录**：公开形态投影。
//!
//! 实现已整体搬到提供商插件（`agent2api-providers`，不随本仓库发布）：网页登录
//! （本机回调 + PKCE）、凭据解析、续期（会轮换 refreshToken）、目录、会话式转发、
//! 额度与签到全在那边。网关这边只剩一件**数据侧**的事：给存量内置记录（没有
//! `pluginSecret` 的那些）拼出与别家同形的公开形态。
//!
//! ── 为什么还要这一支 ────────────────────────────────────────
//! 升级上来的 Trae 账号在库里是内置形状（`accessToken` / `refreshToken` /
//! `deviceId` … 在顶层）。`public_account` 的分派必须有人认它们，否则会掉进
//! WorkBuddy 的兜底形状：界面多出一批 `uid` / `edition` 之类与本家无关的空字段，
//! 而「刷新 Token」按钮靠的 `hasRefreshToken` 只是**恰好**在兜底形状里也算得出来
//! —— 那不是设计，是巧合。
//!
//! 插件新落的账号不走这里（它们有 `pluginSecret`，`account_view` 由插件给）。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use serde_json::{json, Map, Value};

use crate::server::core::account_store::state::StoredAccount;
use crate::server::core::account_store::store::AccountStore;

/// 公开形态（进面板账号列表与 `/api/accounts`）。
///
/// 只读记录上的字段，不解析令牌：到期读数是**落盘值原样**（插件在续期时会把
/// 毫秒写回，存量记录里也可能是秒 —— 面板按毫秒列显示，两种单位历史上都出现过）。
/// 网关不再持有这家的 JWT 解码与单位归一知识，那部分随实现搬到插件了。
fn trae_public_account(record: &StoredAccount) -> Value {
    let access_token = record.access_token();
    let refresh_token = record.refresh_token();
    let mut public = Map::new();
    for key in [
        "id",
        "provider",
        "name",
        "uid",
        "nickname",
        "enterpriseId",
        "source",
        "tokenTail",
        "expiresAt",
        "variant",
    ] {
        public.insert(key.to_string(), record.get(key).cloned().unwrap_or(Value::Null));
    }
    public.insert(
        "expiresAtMs".to_string(),
        Value::from(record.expires_at().map(|value| value as i64).unwrap_or(0)),
    );
    public.insert(
        "hasRefreshToken".to_string(),
        Value::Bool(!refresh_token.trim().is_empty()),
    );
    public.insert("priority".to_string(), Value::from(record.priority()));
    public.insert("enabled".to_string(), Value::Bool(record.enabled()));
    public.insert("addedAt".to_string(), Value::from(record.added_at()));
    public.insert("updatedAt".to_string(), Value::from(record.updated_at()));
    public.insert(
        "proxy".to_string(),
        crate::server::core::proxies::describe_account_proxy(Some(&record.proxy())),
    );
    public.insert(
        "rateLimits".to_string(),
        record.get("rateLimits").cloned().unwrap_or_else(|| json!({})),
    );
    public.insert("desktop".to_string(), Value::Bool(false));
    // 有 accessToken 才算「当前可用」；只有续期串的账号要先刷一次才能用
    public.insert(
        "available".to_string(),
        Value::Bool(!access_token.trim().is_empty()),
    );
    Value::Object(public)
}

impl AccountStore {
    /// 公开形态（`store_view::public_account` 的存量分支调用）。
    pub(crate) fn to_trae_public_account(&self, record: &StoredAccount) -> Value {
        trae_public_account(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 公开形态**不许带令牌本体**：面板只需要尾号与「能不能续期」。
    #[test]
    fn the_public_form_never_carries_tokens() {
        let record = StoredAccount::from_map(
            json!({
                "id": "trae-solo-abc",
                "provider": "trae",
                "uid": "u-1",
                "variant": "solo",
                "accessToken": "A1",
                "refreshToken": "R1",
                "expiresAt": 1_900_000_000_000_i64,
                "tokenTail": "aaaa",
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        let public = trae_public_account(&record);
        assert_eq!("u-1", public.get("uid").and_then(Value::as_str).unwrap_or(""));
        assert_eq!("solo", public.get("variant").and_then(Value::as_str).unwrap_or(""));
        assert_eq!(
            1_900_000_000_000,
            public.get("expiresAtMs").and_then(Value::as_i64).unwrap_or(0)
        );
        assert!(public.get("hasRefreshToken").and_then(Value::as_bool).unwrap_or(false));
        assert!(public.get("available").and_then(Value::as_bool).unwrap_or(false));
        assert!(public.get("accessToken").is_none(), "公开形态不许带令牌");
        assert!(public.get("refreshToken").is_none(), "公开形态不许带续期串");
    }

    /// 只有续期串的存量记录：`available` 为假（要先刷一次才能用），
    /// 但 `hasRefreshToken` 仍为真 —— 「刷新 Token」按钮该亮着。
    #[test]
    fn a_refresh_only_record_is_not_available_but_is_refreshable() {
        let record = StoredAccount::from_map(
            json!({
                "id": "trae-cn-abc",
                "provider": "trae",
                "refreshToken": "R1",
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        let public = trae_public_account(&record);
        assert!(!public.get("available").and_then(Value::as_bool).unwrap_or(true));
        assert!(public.get("hasRefreshToken").and_then(Value::as_bool).unwrap_or(false));
    }
}
