//! CodeArts 账号的**存量投影**（实现已整体搬到提供商插件）。
//!
//! ── 存量展示与配置迁移 ──────────────────────────────────────
//! `CODEARTS_PROVIDER_ID` 保持落盘身份；公开投影服务旧账号，限制器迁移清理已退役
//! 活动的 Token 条件，不改凭证、账号身份和启用状态。
//!
//! ── 为什么投影必须留着 ──────────────────────────────────────
//! 存量记录的凭证散在顶层字段里（`access_key_id` / `security_token` /
//! `oauth_context`…），没有 `pluginSecret` —— `store_view::public_account`
//! 因此走不到「插件记录优先」那一支。不留投影，这些账号会退化成
//! `public_account` 的通用骨架（uid 空、tokenTail 空、`hasRefreshToken` false），
//! 账号页上看起来像坏了。
//!
//! ── 什么已经搬走了 ──────────────────────────────────────────
//! 凭据解析（`Credential::from_payload` / `to_record_value`）、身份判据
//! （`domain_id + user_id`）、续期写回、福利台账、登录落库 —— 全在插件里。
//! 新账号由插件经 `plugin_upsert_account` 落库（凭证在 `pluginSecret`），
//! 上面那条「插件记录优先」会先带走它们，本文件对这些记录是逐字空操作。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use serde_json::{json, Value};

use crate::server::core::limiter::{self, LimiterKind, LimiterRule};
use crate::server::core::limiter::types::Metric;
use crate::server::core::providers::{kind_id, ProviderKind};

use super::state::StoredAccount;

/// 本家的 provider id（`store_view` 的形状分派与别处比对都要用，
/// 与其它几家同一命名：`<X>_PROVIDER_ID`）。
pub const CODEARTS_PROVIDER_ID: &str = kind_id(ProviderKind::CodeArts);

/// 过期活动的 Token 规则不再迁入；已显式清空的限制器保持不限制。
pub(crate) fn migrate_limiters(accounts: &mut [StoredAccount]) -> usize {
    let mut changed = 0;
    for record in accounts {
        if record.provider() != CODEARTS_PROVIDER_ID { continue; }
        let mut groups = limiter::effective_groups_in(record.fields());
        let mut removed = limiter::effective_rules_in(record.fields()).iter().any(|rule| rule.kind == LimiterKind::Token);
        for group in &mut groups {
            let before = group.conditions.len();
            group.conditions.retain(|condition| !matches!(condition.metric, Metric::Tokens | Metric::Requests));
            removed |= group.conditions.len() != before;
        }
        if !removed { continue; }
        groups.retain(|group| !group.conditions.is_empty());
        let configured_empty = if let Some(groups) = record.get("limiterGroups").and_then(Value::as_array) {
            groups.is_empty()
        } else {
            record.get("limiters").and_then(Value::as_array).is_some_and(Vec::is_empty)
        };
        if groups.is_empty() && !configured_empty {
            let rule = LimiterRule { kind: LimiterKind::Balance, action: limiter::LimiterAction::Skip,
                threshold: crate::server::core::usage_records::DEFAULT_LOW_BALANCE_THRESHOLD,
                period: 0, reset: limiter::LimiterReset::Fixed, enabled: true };
            groups = limiter::groups::groups_from_rules(&[rule]);
        }
        for group in &mut groups {
            if let Some(condition) = group.conditions.first_mut() {
                condition.join = limiter::types::Join::Or;
            }
        }
        let rules = limiter::legacy_rules(&groups);
        record.set("limiterGroups", limiter::types::json_of(&groups));
        record.set("limiters", limiter::rules_to_json(&rules));
        let balance = rules.iter().find(|rule| rule.kind == LimiterKind::Balance);
        record.set("lowBalance", balance.map(|rule| json!({
            "mode": if rule.action == limiter::LimiterAction::Disable { "disable" } else { "skip" },
            "threshold": rule.threshold,
        })).unwrap_or_else(|| json!({ "mode": "off", "threshold": 0 })));
        changed += 1;
    }
    changed
}

impl super::AccountStore {
    /// 界面用的公开形态（**只服务存量内置记录**）。
    ///
    /// **绝不带任何凭证字段**（AK/SK/STS/refresh token/DPoP 私钥一个都不出现
    /// 在这里）。
    ///
    /// 判据不解析凭据结构 —— 网关不再认识这家的凭据形状（那部分在插件里）。
    /// 这里只读通用键：`hasRefreshToken` 看 `refreshToken` 在不在，
    /// `available` 看记录有没有 token。缺哪一项就如实报缺，不猜。
    pub(crate) fn to_codearts_public_account(&self, record: &StoredAccount) -> Value {
        let available = record.has_token();
        let can_refresh = !record.refresh_token().is_empty();
        let mut public = serde_json::Map::new();
        for key in ["id", "provider", "name", "userId", "source", "tokenTail", "expiresAt"] {
            public.insert(key.to_string(), record.get(key).cloned().unwrap_or(Value::Null));
        }
        public.insert("edition".to_string(), Value::Null);
        public.insert("editionLabel".to_string(), Value::Null);
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
        // `maxConcurrent` 不在这里写：本家**没有「不限」这个选项**，记录上的 0
        // （含从未设置过）要报成上游的默认并发 3 而不是 0，否则选路读到的 0 等于
        // 不限、不会在本家满员时跳过这个账号（详见 `store_util::
        // max_concurrent_public_for`，那个函数按 provider 套上这层口径）。
        // 进入本家的这个键由 `store_view::public_account` 统一注入。
        //
        // 每日福利领取的台账原样透出（存量记录的顶层键 `codeartsWelfare`；
        // 插件新落的是 `pluginLedger.codeartsWelfare`，那条路走插件投影）：
        // 界面要显示「今天已试过几次 / 是否已到账」。里面**只有活动 id、幂等键
        // 与布尔状态**，没有任何凭据材料，所以不需要像凭据那样过滤。
        public.insert(
            "welfare".to_string(),
            record.get("codeartsWelfare").cloned().unwrap_or(Value::Null),
        );
        // 账号能路由到哪些模型（转发与广告都按这个判），以及它是不是福利通道
        for (key, source) in [
            ("domainId", "domain_id"),
            ("userName", "user_name"),
            ("loginType", "login_type"),
        ] {
            if let Some(value) = record.get(source).filter(|value| !value.is_null()) {
                public.insert(key.to_string(), value.clone());
            }
        }
        Value::Object(public)
    }
}
