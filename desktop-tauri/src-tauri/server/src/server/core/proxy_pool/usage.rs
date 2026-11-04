use std::collections::HashSet;

use serde_json::Value;

use crate::server::config;
use crate::server::core::account_store::AccountStore;

pub fn pool_reference_id(proxy: &Value) -> Option<&str> {
    let proxy = proxy
        .get("config")
        .filter(|value| value.is_object())
        .unwrap_or(proxy);
    if proxy.get("source").and_then(Value::as_str) != Some("pool") {
        return None;
    }
    proxy
        .get("proxyId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
}

pub fn service_references() -> Vec<(&'static str, String)> {
    [("update", config::update_proxy())]
    .into_iter()
    .filter_map(|(service, proxy)| {
        let proxy = proxy?;
        Some((service, pool_reference_id(&proxy)?.to_string()))
    })
    .collect()
}

pub fn referenced_pool_ids(store: Option<&AccountStore>) -> HashSet<String> {
    let mut ids = store
        .map(AccountStore::referenced_pool_ids)
        .unwrap_or_default();
    ids.extend(service_references().into_iter().map(|(_, id)| id));
    ids
}
