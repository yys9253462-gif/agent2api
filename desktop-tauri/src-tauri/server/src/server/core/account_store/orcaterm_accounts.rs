//! OrcaTerm 账号的**公开形态**（本仓库保留的部分）。
//!
//! ── 这份文件为什么只剩一个投影 ──────────────────────────────
//! 本家的凭证解析、`GetUserInfo` 探活、续期、转发全部搬到了**提供商插件**
//! （`agent2api-providers` 的 `orcaterm/`），不随本仓库发布 —— 那是风险隔离
//! 的一部分。新增的账号由插件的 `account.normalize` 归一化后落进
//! `pluginSecret`，公开形态走 `plugin_accounts::to_plugin_public_account`。
//!
//! 留在这里的只有**存量内置记录**（升级上来、还没被插件重写过的那种）的展示
//! 投影：它们没有 `pluginSecret`，凭证还是顶层的 `accessToken` /
//! `tokenExpireAt`，界面需要按本家的字段集显示。这不是上游协议知识，
//! 只是一份字段映射，留在网关不会带来风险。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use serde_json::{Map, Value};

use crate::server::core::account_store::state::StoredAccount;
use crate::server::core::account_store::store::AccountStore;
use crate::server::core::providers::{kind_id, ProviderKind};

/// OrcaTerm provider id（账号存储内部多处要用；**从注册表推导**）。
pub(crate) const ORCATERM_PROVIDER_ID: &str = kind_id(ProviderKind::OrcaTerm);

impl AccountStore {
    // ─── 读：公开形态 ────────────────────────────────────────

    /// 账号的公开形态（前端账号列表 / 会话摘要用；**不含凭证**）。
    ///
    /// 只对**没有** `pluginSecret` 的存量记录生效（判据在
    /// `store_view::public_account`，插件记录先走插件那份投影）。
    pub fn to_orcaterm_public_account(&self, record: &StoredAccount) -> Value {
        let proxy = crate::server::core::proxies::describe_account_proxy(Some(&record.proxy()));
        let mut public = Map::new();
        public.insert("id".to_string(), Value::String(record.id().to_string()));
        public.insert("provider".to_string(), Value::String(record.provider()));
        public.insert("name".to_string(), Value::String(record.name()));
        public.insert(
            "uid".to_string(),
            Value::String(record.get("uid").and_then(Value::as_str).unwrap_or("").to_string()),
        );
        public.insert(
            "loginName".to_string(),
            Value::String(record.get("loginName").and_then(Value::as_str).unwrap_or("").to_string()),
        );
        public.insert(
            "tokenTail".to_string(),
            Value::String(record.get("tokenTail").and_then(Value::as_str).unwrap_or("").to_string()),
        );
        // tokenExpireAt 原样透出（毫秒；0 = 未知）：前端据此显示「到期时间」，
        // 也让「续期是否在工作」可被肉眼校验
        public.insert(
            "tokenExpireAt".to_string(),
            Value::from(record.get("tokenExpireAt").and_then(Value::as_i64).unwrap_or(0)),
        );
        public.insert("desktop".to_string(), Value::Bool(record.is_desktop()));
        public.insert("source".to_string(), Value::String(record.source()));
        public.insert("priority".to_string(), Value::from(record.priority()));
        public.insert("enabled".to_string(), Value::Bool(record.enabled()));
        public.insert("addedAt".to_string(), Value::from(record.added_at()));
        public.insert("updatedAt".to_string(), Value::from(record.updated_at()));
        public.insert("proxy".to_string(), proxy);
        public.insert(
            "rateLimits".to_string(),
            record.get("rateLimits").cloned().unwrap_or_else(|| Value::Object(Map::new())),
        );
        public.insert("available".to_string(), Value::Bool(true));
        Value::Object(public)
    }
}
