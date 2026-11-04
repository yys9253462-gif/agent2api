//! Accio 账号的**存量记录投影**（网关侧只剩这一件事）。
//!
//! ── 这个文件为什么还在 ──────────────────────────────────────
//! 实现已经整体搬到提供商插件（转发 / 目录 / 额度 / 登录 / 续期）。网关不再
//! 解析这家凭证的字段，也不再有自己的添加与续期路径。
//!
//! 但**存量账号的记录形状**还得有人认识：升级上来的库里躺着 `userId` /
//! `accessToken` / `refreshToken` / `mode` 这些顶层字段（没有 `pluginSecret`），
//! 而账号列表要按这家自己的口径把它们渲染出来（`edition` / `editionLabel`
//! 供界面把两地的账号分开，`hasRefreshToken` 决定「刷新 Token」按钮显不显示）。
//!
//! 所以这里只留一个**纯投影**函数：读记录、拼公开形态、不含任何上游知识。
//! 新账号由插件落库（凭证在 `pluginSecret` 里，走 `to_plugin_public_account`）。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use serde_json::{json, Map, Value};

use crate::server::core::providers::accio::endpoints::Region;

use super::state::StoredAccount;
use super::store::AccountStore;

impl AccountStore {
    /// 公开形态（两个地区共用；`edition` / `editionLabel` 体现差异）。
    ///
    /// `available` 的判据是「有令牌、且记录上的 provider 认得出地区」——
    /// 后者用于兜住被改坏 provider 列的记录（它在选路里本来也会被跳过，
    /// 这里如实显示成不可用，比让用户点一个必然失败的账号好）。
    pub fn to_accio_public_account(&self, record: &StoredAccount) -> Value {
        let region = Region::from_provider_id(&record.provider());
        let available = record.has_token() && region.is_some();
        let can_refresh = record
            .get("refreshToken")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty());
        let mut public = Map::new();
        for key in [
            "id",
            "provider",
            "name",
            "userId",
            "email",
            "nickname",
            "source",
            "tokenTail",
            "expiresAt",
        ] {
            public.insert(key.to_string(), record.get(key).cloned().unwrap_or(Value::Null));
        }
        public.insert(
            "edition".to_string(),
            region
                .map(|value| Value::String(value.edition().to_string()))
                .unwrap_or(Value::Null),
        );
        public.insert(
            "editionLabel".to_string(),
            region
                .map(|value| Value::String(value.label().to_string()))
                .unwrap_or(Value::Null),
        );
        public.insert("hasRefreshToken".to_string(), Value::Bool(can_refresh));
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
        public.insert("available".to_string(), Value::Bool(available));
        Value::Object(public)
    }
}
