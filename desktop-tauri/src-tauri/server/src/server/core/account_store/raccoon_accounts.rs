//! 小浣熊账号：**存量记录的公开形态投影**（网关侧只剩这一件事）。
//!
//! ── 为什么这里没有「添加 / 导入 / 刷新回写」了 ─────────────────
//! 小浣熊的实现已整体搬到提供商插件：
//!
//! ```text
//!   凭证解析 / JWT / 桌面端 auth.json   plugins/raccoon/credentials.rs + jwt.rs
//!   添加账号（手动 / 文件 / 桌面端导入）  plugins/raccoon 的 account.normalize
//!   token 续期 + 比较-再写               plugins/raccoon 的 refresh + store.save
//!   网页登录 / 回调换码                  plugins/raccoon/oauth.rs + login.callback
//!   目录 / 余额 / 签到 / 新手任务        plugins/raccoon/{models,balance,onboarding}.rs
//! ```
//!
//! 网关这边**只保留公开形态**：存量内置记录（没有 `pluginSecret`）在列表里
//! 仍要能正常显示 —— 否则用户升级后会看到一排「userId 空、tokenTail 空」的
//! 空行，而账号其实是能用的（装上插件后由插件接管转发）。
//!
//! 新账号由插件写、带 `pluginSecret`，`public_account` 的第一条分支就把它们
//! 交给插件的 `account_view`，**不会走到这里**。
//!
//! ── 硬约束 ────────────────────────────────────────────────
//! 本文件只读已有记录、**没有任何网络请求**（持锁不做网络）。
//! 绝不 unwrap/expect（release 是 panic=abort）。

use serde_json::{Map, Value};

use crate::server::core::account_store::state::StoredAccount;
use crate::server::core::account_store::store::live_desktop_credentials;
use crate::server::core::account_store::store::AccountStore;
use crate::server::core::account_store::store_util::token_tail_of;

impl AccountStore {
    /// 存量小浣熊记录的公开形态（`AccountStore::public_account` 的一支）。
    ///
    /// ── 桌面端账号按**实时值**展示 ─────────────────────────────
    /// 这类账号记录里本来就（按设计）没有 token，`tokenTail` / 过期时间 /
    /// `hasRefreshToken` 若只读记录会永远是空的，界面看起来就是「没有凭证」。
    /// `live_desktop_credentials` 在有桌面端登录态时给出实时值；读不到（客户端
    /// 退出登录、文件被删、或实现已搬到插件）时回落记录里的存量值 —— 于是
    /// 「登录态消失」表现为字段变空，而不是整条记录消失。
    ///
    /// `tokenExpiresAt` 兼容旧数据直接搬过来的 `tokenExpiresAt` 键名。
    pub fn to_raccoon_public_account(&self, record: &StoredAccount) -> Value {
        let proxy = crate::server::core::proxies::describe_account_proxy(Some(
            &record.proxy(),
        ));
        let stored_tail = record
            .get("tokenTail")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let stored_expires = record.expires_at().or_else(|| record.token_expires_at());
        let (token_tail, expires_at, has_refresh) = match live_desktop_credentials(record) {
            Some((token, refresh_token, expires_at)) => (
                if token.is_empty() {
                    stored_tail
                } else {
                    token_tail_of(&token)
                },
                if expires_at > 0.0 { Some(expires_at) } else { stored_expires },
                !refresh_token.is_empty(),
            ),
            None => (
                stored_tail,
                stored_expires,
                !record.refresh_token().is_empty(),
            ),
        };
        let mut public = Map::new();
        public.insert("id".to_string(), Value::String(record.id().to_string()));
        public.insert("provider".to_string(), Value::String(record.provider()));
        public.insert("name".to_string(), Value::String(record.name()));
        public.insert("userId".to_string(), Value::String(record.user_id()));
        public.insert("tokenTail".to_string(), Value::String(token_tail));
        public.insert(
            "tokenExpiresAt".to_string(),
            expires_at
                .map(crate::server::core::account_store::state::json_number)
                .unwrap_or(Value::Null),
        );
        public.insert("hasRefreshToken".to_string(), Value::Bool(has_refresh));
        public.insert("desktop".to_string(), Value::Bool(record.is_desktop()));
        public.insert("source".to_string(), Value::String(record.source()));
        public.insert("priority".to_string(), Value::from(record.priority()));
        public.insert("enabled".to_string(), Value::Bool(record.enabled()));
        public.insert("addedAt".to_string(), Value::from(record.added_at()));
        public.insert("updatedAt".to_string(), Value::from(record.updated_at()));
        public.insert("proxy".to_string(), proxy);
        // `rateLimits` 是**限额冷却标记**（账号×模型），不是积分字段：
        // 选路层（`routing::pick_account_by_priority`）正是从公开形态里读它来决定
        // 「这个账号对该模型是否还在冷却期内」。少了它，小浣熊账号在 429 之后的
        // **下一次请求**仍会被选中（同一次请求内的降级靠 tried_ids 兜住了），
        // 于是客户端重试会一直撞在同一个限额账号上，而不是降级到下一个。
        // 默认给 `{}`（与 workbuddy 的公开形态同一兜底口径）。
        public.insert(
            "rateLimits".to_string(),
            record
                .get("rateLimits")
                .cloned()
                .unwrap_or_else(|| Value::Object(Map::new())),
        );
        // 与 workbuddy 的公开形态同口径：本切片所有账号都视为可用
        public.insert("available".to_string(), Value::Bool(true));
        Value::Object(public)
    }
}
