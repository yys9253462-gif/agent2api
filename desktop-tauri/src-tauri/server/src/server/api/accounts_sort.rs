use axum::body::Bytes;
use axum::response::Response;
use serde_json::Value;

use crate::server::{config, errors, ServerState};
use crate::server::core::{account_order, routing};
use crate::server::http::{ok_json, parse_body};

pub fn get_sorting(state: &ServerState) -> Response {
    let accounts = routing::accounts_of(&state.store().list_visible_accounts());
    ok_json(serde_json::json!(account_order::sorting(&accounts, crate::server::logging::now_ms())))
}

pub fn save_sorting(state: &ServerState, body: &Bytes) -> Response {
    let payload = match parse_body(body) {
        Ok(value) => value,
        Err(error) => return errors::management_error(400, error.message),
    };
    let Some(provider) = payload.get("provider").and_then(Value::as_str) else {
        return errors::management_error(400, "缺少 provider 字段");
    };
    if !crate::server::core::providers::is_known_provider_id(provider)
        && !crate::server::core::custom_providers::is_custom_provider_id(provider)
    {
        return errors::management_error(400, format!("未知提供商「{provider}」"));
    }
    let rules = match payload.get("rules") {
        Some(Value::Null) => None,
        Some(value) => match serde_json::from_value::<Vec<config::AccountSortRule>>(value.clone()) {
            Ok(rules) => Some(rules),
            Err(error) => return errors::management_error(400, format!("账号排序规则格式错误: {error}")),
        },
        None => return errors::management_error(400, "缺少 rules 字段，恢复默认请传 null"),
    };
    if let Err(error) = config::validate_account_sort(provider, rules.as_deref().unwrap_or_default()) {
        return errors::management_error(400, error);
    }
    if let Err(error) = config::save_account_sort(provider, rules, payload.get("onlyIfUnset") == Some(&Value::Bool(true))) {
        return errors::management_error(500, error);
    }
    get_sorting(state)
}
