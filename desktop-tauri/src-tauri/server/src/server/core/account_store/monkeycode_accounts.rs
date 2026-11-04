//! MonkeyCode 账号的**公开形态**与归属探测（本仓库保留的部分）。
//!
//! ── 这份文件为什么这么小 ────────────────────────────────────
//! 本家的 session 探活、`imageId` 发现、模型目录、会话转发全部搬到了**提供商
//! 插件**（`agent2api-providers` 的 `monkeycode/`），不随本仓库发布 —— 那是
//! 风险隔离的一部分。新增账号由插件的 `account.normalize` 归一化后落进
//! `pluginSecret`，公开形态走 `plugin_accounts::to_plugin_public_account`。
//!
//! 留在这里的只有**存量内置记录**的展示投影与归属判定：它们没有
//! `pluginSecret`，凭证还是顶层的 `accessToken` / `imageId` / `userId`，
//! 界面需要按本家的字段集显示（含地区标签）。这不是上游协议知识。
//!
//! ── 地区为什么不落进记录 JSON ────────────────────────────────
//! 地区**已经编码在 provider id 里**（`monkeycode` / `monkeycode-intl`），而
//! 记录自带 `provider` 字段 —— 可由 `Region::from_provider_id` 直接还原。
//! 存两份事实来源会在漂移时静默打到错误的站点。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use serde_json::{json, Map, Value};

use crate::server::core::providers::monkeycode::Region;

use super::state::StoredAccount;
use super::store::AccountStore;

impl AccountStore {
    /// 取一条 MonkeyCode 账号记录（公开形态的原始 JSON）。
    ///
    /// 只服务管理面的**归属判定**（`api::accounts::refresh_account` 用它确认
    /// 「这个 id 属于本地区」）。两个地区是两家 provider，不能互相取到。
    pub fn monkeycode_account_record(&self, region: Region, account_id: &str) -> Option<Value> {
        let guard = self.guard();
        let provider_id = region.provider_id();
        if !account_id.is_empty() {
            let record = self.record_by_id(&guard, account_id)?;
            return (record.provider() == provider_id).then(|| record.to_value());
        }
        let mut candidates: Vec<StoredAccount> = self
            .records_for_provider(&guard, provider_id)
            .into_iter()
            .filter(StoredAccount::enabled)
            .collect();
        candidates.sort_by_key(StoredAccount::order_key);
        candidates.into_iter().next().map(|item| item.to_value())
    }

    /// MonkeyCode 账号的**公开形态**（进 HTTP 响应）：去掉 session 本体，只留
    /// 尾 4 位；带上 `imageId` 与 `edition`（地区）供界面显示。
    ///
    /// 只对**没有** `pluginSecret` 的存量记录生效（判据在
    /// `store_view::public_account`，插件记录先走插件那份投影）。
    pub fn to_monkeycode_public_account(&self, record: &StoredAccount) -> Value {
        let region = Region::from_provider_id(&record.provider());
        let available = record.has_token() && region.is_some();
        let mut public = Map::new();
        for key in ["id", "provider", "name", "userId", "source", "tokenTail", "imageId"] {
            public.insert(
                key.to_string(),
                record.get(key).cloned().unwrap_or(Value::Null),
            );
        }
        public.insert(
            "edition".to_string(),
            region
                .map(|value| Value::String(value.provider_id().to_string()))
                .unwrap_or(Value::Null),
        );
        public.insert(
            "editionLabel".to_string(),
            region
                .map(|value| Value::String(value.label().to_string()))
                .unwrap_or(Value::Null),
        );
        // 有没有 image_id：没有时界面上「创建任务」注定失败，前端据此提示先补
        public.insert(
            "hasImageId".to_string(),
            Value::Bool(
                record
                    .get("imageId")
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty()),
            ),
        );
        // 本家没有续期协议（30 天硬限制），恒 false
        public.insert("hasRefreshToken".to_string(), Value::Bool(false));
        public.insert("desktop".to_string(), Value::Bool(false));
        public.insert("expiresAt".to_string(), Value::from(record.expires_at().unwrap_or(0.0) as i64));
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
        public.insert("available".to_string(), Value::Bool(available));
        Value::Object(public)
    }
}
