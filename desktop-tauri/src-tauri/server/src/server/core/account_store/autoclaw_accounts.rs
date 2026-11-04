//! AutoClaw 账号：**存量记录的公开形态投影**（网关侧只剩这一件事）。
//!
//! ── 为什么这里没有「添加 / 导入 / 刷新回写」了 ─────────────────
//! AutoClaw 的实现已整体搬到提供商插件：
//!
//! ```text
//!   凭证解析 / enc: 解密 / 桌面端 auth.json   plugins/autoclaw/credentials.rs + crypto.rs
//!   添加账号（手动 / 文件 / 桌面端导入）        plugins/autoclaw 的 account.normalize
//!   token 续期 + 比较-再写                    plugins/autoclaw/refresh.rs + store::save
//!   短信 / OAuth 登录落库                     plugins/autoclaw/login.rs + web_login.rs
//! ```
//!
//! 插件新落的账号带 `pluginSecret`，公开形态由插件的 `account_view` 给
//! （`store_view::public_account` 的第一条分支）。**本文件只服务存量记录** ——
//! 那些记录是拆家之前写的，没有 `pluginSecret`，走不到插件那条路。
//!
//! ── 存量投影为什么读不到实时桌面端登录态 ──────────────────────
//! 桌面端记录（`autoclaw-desktop` / `autoclaw-intl-desktop`）设计上不落 token，
//! 凭证靠实时读 `%APPDATA%/AutoClaw/auth.json` 并做 safeStorage 解密 —— 那条
//! 链（DPAPI + AES-GCM）是**上游客户端知识**，已随实现搬走。因此存量桌面记录
//! 在界面上的 token 尾号 / 过期时间会是空的、`available` 如实报 false，等用户
//! 装上插件后重新导入一次即可恢复（插件会建一条带 `pluginSecret` 的新记录）。
//!
//! ── 两个固定 id 为什么还留在这里 ────────────────────────────
//! 它们是**落盘契约**：存量记录仍在库里，导入 / 导出时的「保留 id」判定
//! （`account_transfer::identity::RESERVED_DESKTOP_IDS`）必须认得它们，
//! 否则一条「带桌面 id 但不带 desktop 标记」的记录会占住 id，让后续的桌面端
//! 导入建不出实时账号。
//!
//! ── 硬约束 ────────────────────────────────────────────────
//! 本文件全是纯读，**没有任何网络请求**；绝不 unwrap/expect。

use serde_json::{Map, Value};

use crate::server::core::account_store::state::StoredAccount;
use crate::server::core::account_store::{is_autoclaw_family, AccountStore};
use crate::server::core::providers::autoclaw::Region;

/// 国内版桌面端账号的固定 id（历史落盘值）。
pub const DESKTOP_ACCOUNT_ID: &str = "autoclaw-desktop";

/// 国际版桌面端账号的固定 id（与国内版**必须不同**：两地共用同一个 auth.json，
/// 但账号记录是两条独立记录；同 id 会撞唯一约束）。
pub const INTL_DESKTOP_ACCOUNT_ID: &str = "autoclaw-intl-desktop";

/// 按地区取桌面端记录的固定 id。
pub fn desktop_account_id(region: Region) -> &'static str {
    match region {
        Region::Cn => DESKTOP_ACCOUNT_ID,
        Region::Intl => INTL_DESKTOP_ACCOUNT_ID,
    }
}

impl AccountStore {
    /// AutoClaw 存量账号的公开形态（对照原项目 `toPublicAccount`）。
    ///
    /// 字段：`id` / `provider` / `name` / `userId` / `email` / `deviceId` /
    /// `tokenTail` / `tokenExpiresAt` / `hasRefreshToken` / `desktop` / `source` /
    /// `priority` / `enabled` / `addedAt` / `updatedAt` / `proxy` / `rateLimits` /
    /// `available`。
    ///
    /// ── 只读记录，不做任何实时解析 ─────────────────────────────
    /// 拆家之后网关不认识 auth.json 的格式、也不知道怎么解密它（那是插件的事）。
    /// 因此桌面端记录的 `tokenTail` / `tokenExpiresAt` 只能是记录里的存量值
    /// （通常为空），`available` 按「记录里有没有可用 token」如实报。
    ///
    /// ── `email` 为什么缺省不写键 ───────────────────────────────
    /// 界面拿它当副标题；读不到邮箱的账号（国内版的手机号账号等）应当**没有**
    /// 那一行，而不是显示一个空行。
    pub fn to_autoclaw_public_account(&self, record: &StoredAccount) -> Value {
        debug_assert!(
            is_autoclaw_family(&record.provider()),
            "to_autoclaw_public_account 只接受 AutoClaw 系记录"
        );
        let proxy = crate::server::core::proxies::describe_account_proxy(Some(&record.proxy()));
        let token_tail = record
            .get("tokenTail")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let expires_at = record.expires_at().or_else(|| record.token_expires_at());
        let email = record
            .get("email")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        let device_id = record
            .get("deviceId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        // 桌面端记录按设计不落 token：可用性只能按「记录里有没有令牌」判，
        // 实时登录态由插件自己去读（见模块头）。
        let available = record.has_token() || !record.access_token().is_empty();
        let mut public = Map::new();
        public.insert("id".to_string(), Value::String(record.id().to_string()));
        public.insert("provider".to_string(), Value::String(record.provider()));
        public.insert("name".to_string(), Value::String(record.name()));
        public.insert("userId".to_string(), Value::String(record.user_id()));
        if !email.is_empty() {
            public.insert("email".to_string(), Value::String(email));
        }
        public.insert("deviceId".to_string(), Value::String(device_id));
        public.insert("tokenTail".to_string(), Value::String(token_tail));
        public.insert(
            "tokenExpiresAt".to_string(),
            expires_at
                .map(crate::server::core::account_store::state::json_number)
                .unwrap_or(Value::Null),
        );
        public.insert(
            "hasRefreshToken".to_string(),
            Value::Bool(!record.refresh_token().is_empty()),
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
            record
                .get("rateLimits")
                .cloned()
                .unwrap_or_else(|| Value::Object(Map::new())),
        );
        public.insert("available".to_string(), Value::Bool(available));
        Value::Object(public)
    }
}
