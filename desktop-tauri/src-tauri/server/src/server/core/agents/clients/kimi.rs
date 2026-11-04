use super::super::{Model, Profile};
use super::{native_entry, native_id, set, string};
use serde_json::{json, Value};
use std::path::Path;

pub fn read(value: &Value) -> Result<Vec<Profile>, String> {
    if !value.is_object() {
        return Err("Kimi Code 配置必须是表".into());
    }
    let selected = string(value, "default_model");
    let models = value.get("models").and_then(Value::as_object);
    let providers = value.get("providers").and_then(Value::as_object);
    let mut out = Vec::new();
    if let Some(providers) = providers {
        for (id, entry) in providers {
            let base = string(entry, "base_url");
            let protocol = match string(entry, "type").as_str() {
                "kimi" | "openai" | "openai_legacy" => "openai_chat",
                "openai_responses" => "responses",
                "anthropic" => "anthropic",
                _ => "unsupported",
            };
            let mut profile = Profile::new(id, id, protocol, &base);
            profile.enabled = false;
            profile.default = false;
            profile.native.entry = entry.clone();
            profile.api_key = string(entry, "api_key");
            let mut native_models = serde_json::Map::new();
            if let Some(models) = models {
                for (key, entry) in models
                    .iter()
                    .filter(|(_, entry)| entry.get("provider").and_then(Value::as_str) == Some(id))
                {
                    let model_id = string(entry, "model");
                    profile.models.push(Model {
                        id: model_id.clone(),
                        name: string(entry, "display_name"),
                        context_window: entry.get("max_context_size").and_then(Value::as_u64),
                        max_output_tokens: entry.get("max_output_size").and_then(Value::as_u64),
                        supports_images: entry
                            .get("capabilities")
                            .and_then(Value::as_array)
                            .map(|v| v.iter().any(|v| v == "image_in")),
                        ..Default::default()
                    });
                    native_models.insert(key.clone(), entry.clone());
                    if key == &selected {
                        profile.enabled = true;
                        profile.default = true;
                        profile.model = model_id;
                    }
                }
            }
            if profile.model.is_empty() {
                profile.model = profile
                    .models
                    .first()
                    .map(|m| m.id.clone())
                    .unwrap_or_default();
            }
            profile.native.model_entries = Value::Object(native_models);
            out.push(profile);
        }
    }
    Ok(out)
}

pub fn write(
    path: &Path,
    value: &mut Value,
    profile: &Profile,
    operation: &str,
) -> Result<(), String> {
    let id = native_id(profile);
    let mut provider = native_entry(
        value.get("providers").and_then(|v| v.get(&id)),
        &profile.native.entry,
    )?;
    let protocol = match profile.protocol.as_str() {
        "anthropic" => "anthropic",
        "responses" => "openai_responses",
        _ => {
            if super::super::paths::kimi_legacy(path) {
                "openai_legacy"
            } else {
                "openai"
            }
        }
    };
    set(&mut provider, &["type"], json!(protocol))?;
    set(&mut provider, &["base_url"], json!(profile.base_url))?;
    super::apply_credential(&mut provider, &["api_key"], profile)?;
    if profile.auth_mode != "preserve" {
        super::remove(&mut provider, &["api_key_env"]);
        super::remove(&mut provider, &["oauth"]);
    }
    set(value, &["providers", &id], provider)?;
    let imported = profile.native.model_entries.as_object();
    let mut default_key = String::new();
    for model in &profile.models {
        let original_key = imported
            .and_then(|items| {
                items.iter().find(|(_, entry)| {
                    entry.get("model").and_then(Value::as_str) == Some(&model.id)
                })
            })
            .map(|(key, _)| key.clone());
        let key = original_key.unwrap_or_else(|| format!("{id}/{}", model.id));
        let existing = value.get("models").and_then(|v| v.get(&key));
        let original = imported.and_then(|v| v.get(&key)).unwrap_or(&Value::Null);
        let mut entry = native_entry(existing, original)?;
        set(&mut entry, &["provider"], json!(id))?;
        set(&mut entry, &["model"], json!(model.id))?;
        set(
            &mut entry,
            &["display_name"],
            json!(if model.name.is_empty() {
                &model.id
            } else {
                &model.name
            }),
        )?;
        if let Some(v) = model.context_window {
            set(&mut entry, &["max_context_size"], json!(v))?;
        }
        if !super::super::paths::kimi_legacy(path) {
            if let Some(v) = model.max_output_tokens {
                set(&mut entry, &["max_output_size"], json!(v))?;
            }
        }
        if let Some(images) = model.supports_images {
            let mut caps = entry
                .get("capabilities")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            caps.retain(|v| v != "image_in");
            if images {
                caps.push(json!("image_in"));
            }
            set(&mut entry, &["capabilities"], json!(caps))?;
        }
        set(value, &["models", &key], entry)?;
        if model.id == profile.model {
            default_key = key;
        }
    }
    if operation == "default" {
        set(value, &["default_model"], json!(default_key))?;
    }
    Ok(())
}
