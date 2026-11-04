//! KukuAI 账号的**存量记录投影**（网关侧只剩这一件事）。
//!
//! ── 这个文件为什么还在 ──────────────────────────────────────
//! 实现已经整体搬到提供商插件（凭证解析 / 本机客户端读取 / 网页登录 /
//! 会话换发 / 转发 / 目录 / 余额 / 签到）。
//!
//! 但**存量账号的记录形状**还得有人认识：升级上来的库里躺着 `uid` /
//! `loginName` / `tokenTail` / `desktop` 这些顶层字段（没有 `pluginSecret`），
//! 而账号列表要按这家自己的口径把它们渲染出来（`desktop` 决定「桌面端」徽章，
//! `source` 标着这条是粘贴 / 文件 / 本机导入）。
//!
//! 所以这里只留一个**纯投影**函数：读记录、拼公开形态、不含任何上游知识。
//! 新账号由插件落库（凭证在 `pluginSecret` 里，走 `to_plugin_public_account`）。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use serde_json::{Map, Value};

use crate::server::core::providers::{kind_id, ProviderKind};

use super::state::StoredAccount;
use super::store::AccountStore;

/// KukuAI 的 provider id（`store_view` 的分派键）。
pub(crate) const KUKU_PROVIDER_ID: &str = kind_id(ProviderKind::Kuku);

impl AccountStore {
    /// 账号的公开形态（前端账号列表 / 会话摘要用；**不含凭证**）。
    pub fn to_kuku_public_account(&self, record: &StoredAccount) -> Value {
        let proxy = crate::server::core::proxies::describe_account_proxy(Some(&record.proxy()));
        let mut public = Map::new();
        public.insert("id".to_string(), Value::String(record.id().to_string()));
        public.insert("provider".to_string(), Value::String(record.provider()));
        public.insert("name".to_string(), Value::String(record.name()));
        for key in ["uid", "loginName", "tokenTail"] {
            public.insert(
                key.to_string(),
                Value::String(record.get(key).and_then(Value::as_str).unwrap_or("").to_string()),
            );
        }
        public.insert("desktop".to_string(), Value::Bool(record.is_desktop()));
        public.insert("source".to_string(), Value::String(record.source()));
        public.insert("priority".to_string(), Value::from(record.priority()));
        public.insert("enabled".to_string(), Value::Bool(record.enabled()));
        public.insert("addedAt".to_string(), Value::from(record.added_at()));
        public.insert("updatedAt".to_string(), Value::from(record.updated_at()));
        public.insert("proxy".to_string(), proxy);
        public.insert(
            "rateLimits".to_string(),
            record
                .get("rateLimits")
                .cloned()
                .unwrap_or_else(|| Value::Object(Map::new())),
        );
        public.insert("available".to_string(), Value::Bool(true));
        Value::Object(public)
    }
}
