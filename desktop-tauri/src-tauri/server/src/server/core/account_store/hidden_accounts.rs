use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{json, Value};

use crate::server::core::plugin_host::PackRegistry;
use crate::server::core::providers;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum HiddenReason {
    NotInstalled,
    PackDisabled,
    ProviderDisabled,
    NotLoaded,
}

fn reason_of(provider: &str, registry: &PackRegistry) -> HiddenReason {
    let mut reason = HiddenReason::NotInstalled;
    for pack in &registry.packs {
        if !pack
            .manifest_providers
            .iter()
            .any(|item| item.id.as_str() == provider)
        {
            continue;
        }
        if pack.enabled {
            if pack.providers.get(provider).copied().unwrap_or(true) {
                return HiddenReason::NotLoaded;
            }
            reason = HiddenReason::ProviderDisabled;
        } else if matches!(reason, HiddenReason::NotInstalled) {
            reason = HiddenReason::PackDisabled;
        }
    }
    reason
}

/// 只返回数量和提供商元数据，不返回隐藏账号的记录或凭证。
pub(super) fn summary(counts: BTreeMap<String, usize>) -> Value {
    let total = counts.values().sum::<usize>();
    let mut details = Vec::with_capacity(counts.len());
    if !counts.is_empty() {
        let registry = PackRegistry::load();
        for (provider, count) in &counts {
            details.push(json!({
                "id": provider,
                "label": providers::label_of(provider),
                "count": count,
                "reason": reason_of(provider, &registry),
            }));
        }
    }
    json!({
        "total": total,
        "byProvider": counts,
        "providers": details,
    })
}
