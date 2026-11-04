//! 插件化之前的目录缓存兼容：scope 属于存储契约，旧条目转换为插件目录形态。

use serde_json::Value;

use super::catalog_cache::{self, CachedCatalog};
use super::cline::{models, Pool};
use super::{kind_from_id, ProviderKind};

pub(super) fn load(provider: &str) -> Option<CachedCatalog> {
    let kind = kind_from_id(provider)?;
    let scope = match kind {
        ProviderKind::WorkBuddy => "workbuddy",
        ProviderKind::WorkBuddyIntl => "workbuddyIntl",
        ProviderKind::Raccoon => "raccoon",
        ProviderKind::CatPaw => "catpaw",
        ProviderKind::AutoClaw => "autoclawCn",
        ProviderKind::AutoClawIntl => "autoclawIntl",
        ProviderKind::Qoder => "qoderCn",
        ProviderKind::QoderIntl => "qoderGlobal",
        ProviderKind::CodeArts => "codearts",
        ProviderKind::ClineFree | ProviderKind::ClinePass => "cline",
        ProviderKind::Accio => "accioGlobal",
        ProviderKind::AccioCn => "accioCn",
        ProviderKind::Trae => "trae",
        ProviderKind::Loomy => "loomy",
        ProviderKind::MonkeyCode => "monkeycodeCn",
        ProviderKind::MonkeyCodeIntl => "monkeycodeIntl",
        ProviderKind::CommandCode => "commandcode",
        ProviderKind::Antigravity => "antigravity",
        ProviderKind::OfficeAce => "officeace",
        ProviderKind::Chatglm => "chatglm",
        ProviderKind::Modelscope => "modelscopeCn",
        ProviderKind::ModelscopeIntl => "modelscopeIntl",
        ProviderKind::Atomcode => "atomcode",
        ProviderKind::LobsterAi => "lobsterai",
        _ => return None,
    };
    let mut cached = catalog_cache::load(scope)?;
    cached.models = cached
        .models
        .into_iter()
        .filter_map(|model| normalize(kind, model))
        .collect();
    (!cached.models.is_empty()).then_some(cached)
}

fn copy_field(object: &mut serde_json::Map<String, Value>, source: &str, target: &str) {
    if !object.get(target).is_some_and(|value| !value.is_null()) {
        if let Some(value) = object.get(source).cloned() {
            object.insert(target.to_string(), value);
        }
    }
}

fn normalize(kind: ProviderKind, mut model: Value) -> Option<Value> {
    let object = model.as_object_mut()?;
    match kind {
        ProviderKind::Raccoon => {
            if !object
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| !id.trim().is_empty())
            {
                let id = object
                    .get("name")
                    .and_then(Value::as_str)?
                    .trim()
                    .to_string();
                object.insert("id".to_string(), Value::String(id.clone()));
                let name = object
                    .get("displayName")
                    .cloned()
                    .unwrap_or(Value::String(id));
                object.insert("name".to_string(), name);
            }
            for (source, target) in [
                ("contextWindow", "maxInputTokens"),
                ("maxTokens", "maxOutputTokens"),
                ("supportImage", "supportsImages"),
                ("supportThinking", "supportsReasoning"),
            ] {
                copy_field(object, source, target);
            }
        }
        ProviderKind::ClineFree | ProviderKind::ClinePass => {
            let id = object.get("id").and_then(Value::as_str)?.trim();
            if id.starts_with(models::CLOUD_PREFIX) {
                return None;
            }
            // 旧槽包含两池；免费池也有无前缀模型，不能只凭 id 重新归池。
            let pool = object
                .get("pool")
                .and_then(Value::as_str)
                .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
                    "free" | "cline-free" => Some(Pool::Free),
                    "pass" | "cline-pass" => Some(Pool::Pass),
                    _ => None,
                })
                .unwrap_or_else(|| models::pool_of(id));
            let expected = if kind == ProviderKind::ClineFree {
                Pool::Free
            } else {
                Pool::Pass
            };
            if pool != expected {
                return None;
            }
            copy_field(object, "contextWindow", "maxInputTokens");
            copy_field(object, "reasoning", "supportsReasoning");
            object
                .entry("supportsToolCall")
                .or_insert(Value::Bool(true));
            object.entry("isDefault").or_insert(Value::Bool(false));
            object
                .entry("kind")
                .or_insert(Value::String("chat".to_string()));
            for key in ["pool", "contextWindow", "reasoning"] {
                object.remove(key);
            }
        }
        ProviderKind::Chatglm => {
            copy_field(object, "selected_model", "id");
            copy_field(object, "display_name", "name");
            copy_field(object, "reasoning_efforts", "reasoningEfforts");
        }
        _ => {}
    }
    Some(model)
}
