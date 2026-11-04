use std::sync::Arc;

use serde_json::{Map, Value};

use crate::server::core::sanitize::{self, RuleSet, SanitizeRule};

use super::{update, RuntimeConfig, KEY_SANITIZE_FINGERPRINTS};

pub const KEY_SANITIZE_RULES: &str = "sanitizeFingerprintRules";
pub const KEY_SANITIZE_PROVIDERS: &str = "sanitizeFingerprintProviders";
pub const MAX_SANITIZE_PROVIDERS: usize = 200;

#[derive(Clone, Debug)]
pub struct SanitizeSettings {
    pub enabled: bool,
    pub rules: Arc<RuleSet>,
    /// 空列表表示全部提供商，包含之后新增的插件与自定义提供商。
    pub providers: Vec<String>,
    pub custom_rules: bool,
}

impl Default for SanitizeSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            rules: sanitize::default_rules(),
            providers: Vec::new(),
            custom_rules: false,
        }
    }
}

impl SanitizeSettings {
    pub fn applies_to(&self, provider_id: &str) -> bool {
        self.enabled
            && (self.providers.is_empty()
                || self
                    .providers
                    .iter()
                    .any(|id| id.eq_ignore_ascii_case(provider_id)))
    }
}

#[derive(Default)]
pub struct SanitizePatch {
    pub enabled: Option<bool>,
    /// None = 不改，Some(None) = 恢复内置规则，Some(Some(...)) = 自定义词表。
    pub rules: Option<Option<Arc<RuleSet>>>,
    pub providers: Option<Vec<String>>,
}

pub(super) fn from_raw(raw: &Map<String, Value>) -> SanitizeSettings {
    let parsed = raw
        .get(KEY_SANITIZE_RULES)
        .filter(|value| !value.is_null())
        .and_then(|value| serde_json::from_value::<Vec<SanitizeRule>>(value.clone()).ok())
        .and_then(|entries| RuleSet::compile(entries).ok());
    let custom_rules = parsed.is_some();
    let mut providers: Vec<String> = raw
        .get(KEY_SANITIZE_PROVIDERS)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|id| id.trim().to_ascii_lowercase())
        .filter(|id| !id.is_empty())
        .collect();
    providers.sort();
    providers.dedup();
    SanitizeSettings {
        enabled: raw
            .get(KEY_SANITIZE_FINGERPRINTS)
            .and_then(Value::as_bool)
            .unwrap_or(true),
        rules: parsed.map(Arc::new).unwrap_or_else(sanitize::default_rules),
        providers,
        custom_rules,
    }
}

impl RuntimeConfig {
    pub fn sanitize_settings(&self) -> &SanitizeSettings {
        &self.sanitize
    }
}

pub fn set_sanitize_settings(patch: SanitizePatch) -> bool {
    update(|config| {
        if let Some(enabled) = patch.enabled {
            config
                .raw
                .insert(KEY_SANITIZE_FINGERPRINTS.to_string(), Value::Bool(enabled));
            config.sanitize.enabled = enabled;
        }
        if let Some(rules) = patch.rules {
            match rules {
                Some(rules) => {
                    let entries =
                        serde_json::to_value(rules.entries()).expect("脱敏词表必须可序列化");
                    config.raw.insert(KEY_SANITIZE_RULES.to_string(), entries);
                    config.sanitize.rules = rules;
                    config.sanitize.custom_rules = true;
                }
                None => {
                    config.raw.remove(KEY_SANITIZE_RULES);
                    config.sanitize.rules = sanitize::default_rules();
                    config.sanitize.custom_rules = false;
                }
            }
        }
        if let Some(providers) = patch.providers {
            config.raw.insert(
                KEY_SANITIZE_PROVIDERS.to_string(),
                Value::from(providers.clone()),
            );
            config.sanitize.providers = providers;
        }
    })
}
