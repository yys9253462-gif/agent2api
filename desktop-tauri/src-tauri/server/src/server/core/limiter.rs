//! 账号限制器：条件组、提供商额度来源与独立本地用量台账。

pub mod counter;
pub mod evaluate;
pub mod groups;
mod legacy;
pub mod sources;
pub mod types;

pub use groups::{describe_groups, effective_groups, effective_groups_in, legacy_rules, normalize_groups};
pub use legacy::{
    balance_disable_threshold, balance_skip_blocked, default_rules, effective_rules,
    effective_rules_in, effective_rules_json_in, normalize_rules, rules_to_json, describe_rules,
    LimiterAction, LimiterKind, LimiterReset, LimiterRule, TokenFact,
    enabled_token_kinds, refresh_token_facts, token_facts, token_skip_blocked,
    token_disable_hit, token_usage_rows, describe_period,
};

use serde_json::Value;
use crate::server::{core::account_store::AccountStore, db::Db, logging};

pub fn install(db: Option<Db>) {
    legacy::install(db.clone());
    counter::install(db);
    sources::reload();
}

pub fn note_request_tokens(account_id: &str, tokens: i64, now_ms: i64) {
    legacy::note_request_tokens(account_id, tokens, now_ms);
}

pub fn refresh(accounts: &[Value]) { counter::refresh(accounts); }

pub fn enforce_disable(store: &AccountStore, accounts: &[Value]) {
    let sources = sources::facts();
    let counts = counter::facts();
    let now = logging::now_ms();
    for account in accounts {
        if account.get("enabled").and_then(Value::as_bool) == Some(false) { continue; }
        if !evaluate::disable_hit(account, &sources, &counts, now) { continue; }
        let Some(id) = account.get("id").and_then(Value::as_str) else { continue };
        let name = account.get("displayName").or_else(|| account.get("name"))
            .and_then(Value::as_str).unwrap_or(id);
        match store.update_account(id, &serde_json::json!({ "enabled": false })) {
            Ok(_) => logging::log_with_level("[Usage]", &format!("账号「{name}」限制条件组已触发，已自动禁用（需手动重新启用）"), "warn"),
            Err(error) => logging::verbose("[Usage]", &format!("账号 {id} 限制器自动禁用失败：{}", error.message)),
        }
    }
}
