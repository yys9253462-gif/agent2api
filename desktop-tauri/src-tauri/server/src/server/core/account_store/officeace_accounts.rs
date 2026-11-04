//! OfficeAce 账号的**存量记录投影**（网关侧只剩这一件事）。
//!
//! ── 这个文件为什么还在 ──────────────────────────────────────
//! 实现已经整体搬到提供商插件（转发 / 目录 / 额度 / 签到 / 登录 / 续期）。
//! 网关不再解析这家凭证的任何字段，也不再有它的添加与续期路径 —— 那些
//! 都是「怎么跟上游打交道」的知识，留在这里就等于主仓库继续携带反向代理
//! 的实现细节。
//!
//! 但**存量账号的记录形状**还得有人认识：升级上来的库里躺着 `baseUrl` /
//! `modelAppKey` / `modelAppSecret` / `expiresAt` 这些顶层字段（没有
//! `pluginSecret`），而账号列表要按这家自己的口径把它们渲染出来
//! （`baseUrl` 给面板改凭据用、`expiresAt` 给域配置的 `expiry` 读、
//! `hasRefreshToken` 决定「刷新 Token」按钮显不显示）。
//!
//! 所以这里只留一个**纯投影**函数：读记录、拼公开形态、不含任何上游知识。
//! 新账号由插件落库（凭证在 `pluginSecret` 里，走 `to_plugin_public_account`），
//! 不会走到这里。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use serde_json::{Map, Value};

use crate::server::core::account_store::state::StoredAccount;
use crate::server::core::account_store::{AccountStore, OFFICEACE_PROVIDER_ID};

impl AccountStore {
    /// 取一条 OfficeAce 账号记录（存量记录形态的原始 JSON）。
    ///
    /// `account_id` 非空 → 按 id 直查（不属于本家返回 None）；
    /// 为空 → 本家**启用中**账号里优先级最靠前的一条。
    ///
    /// ── 为什么网关还需要它 ──────────────────────────────────────
    /// 账号转移与「这条记录归哪家」的判定要能按 id 认领归属；插件家的记录
    /// 同样满足（`provider` 列就是归属），所以这条对两种形态都成立。
    pub fn officeace_account_record(&self, account_id: &str) -> Option<Value> {
        let guard = self.guard();
        if !account_id.is_empty() {
            let record = self.record_by_id(&guard, account_id)?;
            return (record.provider() == OFFICEACE_PROVIDER_ID).then(|| record.to_value());
        }
        let mut candidates: Vec<StoredAccount> = self
            .records_for_provider(&guard, OFFICEACE_PROVIDER_ID)
            .into_iter()
            .filter(StoredAccount::enabled)
            .collect();
        candidates.sort_by_key(StoredAccount::order_key);
        candidates.into_iter().next().map(|item| item.to_value())
    }

    /// 账号的公开形态（前端账号列表 / 会话摘要用；**不含任何凭据**）。
    ///
    /// 字段口径照 kuku 那份：标识 + 归属 + 队列状态 + 时间戳 + 代理描述，
    /// 再加本家界面真正会读的三样 —— `baseUrl`（面板改凭据时要用）、
    /// `expiresAt`（控制面临时凭据到期时刻，域配置里 `expiry:'expiresAt'` 读它）、
    /// `hasRefreshToken`（面板「刷新 Token」按钮的开关）。
    pub fn to_officeace_public_account(&self, record: &StoredAccount) -> Value {
        let proxy = crate::server::core::proxies::describe_account_proxy(Some(&record.proxy()));
        let mut public = Map::new();
        public.insert("id".to_string(), Value::String(record.id().to_string()));
        public.insert("provider".to_string(), Value::String(record.provider()));
        public.insert("name".to_string(), Value::String(record.name()));
        public.insert("source".to_string(), Value::String(record.source()));
        public.insert("priority".to_string(), Value::from(record.priority()));
        public.insert("enabled".to_string(), Value::Bool(record.enabled()));
        public.insert("addedAt".to_string(), Value::from(record.added_at()));
        public.insert("updatedAt".to_string(), Value::from(record.updated_at()));
        public.insert("proxy".to_string(), proxy);
        public.insert(
            "baseUrl".to_string(),
            record.get("baseUrl").cloned().unwrap_or(Value::Null),
        );
        public.insert(
            "expiresAt".to_string(),
            record.get("expiresAt").cloned().unwrap_or(Value::Null),
        );
        public.insert(
            "hasRefreshToken".to_string(),
            Value::Bool(
                record
                    .get("refreshToken")
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty()),
            ),
        );
        Value::Object(public)
    }
}
