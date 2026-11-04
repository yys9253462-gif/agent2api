use super::super::{
    files::{self, Change},
    ConfigFormat, Model, Profile,
};
use super::{merge, native_entry, native_id, set, string};
use serde_json::{json, Value};
use std::path::Path;

pub fn read(value: &Value) -> Result<Vec<Profile>, String> {
    if !value.is_object() {
        return Err("Crush JSON 配置必须是对象".into());
    }
    let active = value
        .pointer("/models/large/provider")
        .and_then(Value::as_str)
        .unwrap_or("");
    let current = value
        .pointer("/models/large/model")
        .and_then(Value::as_str)
        .unwrap_or("");
    let mut out = Vec::new();
    if let Some(providers) = value.get("providers").and_then(Value::as_object) {
        for (id, entry) in providers {
            let base = string(entry, "base_url");
            if base.is_empty() {
                continue;
            }
            let provider_type = string(entry, "type");
            let protocol = match provider_type.as_str() {
                "openai-compat" => "openai_chat",
                "openai" => "responses",
                "anthropic" => "anthropic",
                other => other,
            };
            let mut profile = Profile::new(id, &string(entry, "name"), protocol, &base);
            profile.api_key = string(entry, "api_key");
            profile.enabled = id == active;
            profile.default = profile.enabled;
            profile.native.entry = entry.clone();
            if let Some(models) = entry.get("models").and_then(Value::as_array) {
                for entry in models {
                    profile.models.push(Model {
                        id: string(entry, "id"),
                        name: string(entry, "name"),
                        context_window: entry.get("context_window").and_then(Value::as_u64),
                        max_output_tokens: entry.get("default_max_tokens").and_then(Value::as_u64),
                        supports_images: entry
                            .get("supports_attachments")
                            .or_else(|| entry.get("supports_images"))
                            .and_then(Value::as_bool),
                        supports_reasoning: entry.get("can_reason").and_then(Value::as_bool),
                        reasoning_levels: entry
                            .get("reasoning_levels")
                            .and_then(Value::as_array)
                            .map(|v| {
                                v.iter()
                                    .filter_map(Value::as_str)
                                    .map(str::to_string)
                                    .collect()
                            })
                            .unwrap_or_default(),
                        effort: string(entry, "default_reasoning_effort"),
                        ..Default::default()
                    });
                }
            }
            profile.model = if profile.enabled {
                current.into()
            } else {
                profile
                    .models
                    .first()
                    .map(|m| m.id.clone())
                    .unwrap_or_default()
            };
            if profile.enabled {
                profile.effort = value
                    .pointer("/models/large/reasoning_effort")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .into();
            }
            out.push(profile);
        }
    }
    Ok(out)
}

pub(super) fn data_path() -> std::path::PathBuf {
    if cfg!(windows) {
        std::env::var_os("CRUSH_GLOBAL_DATA")
            .or_else(|| {
                std::env::var_os("LOCALAPPDATA")
                    .map(|v| Path::new(&v).join("crush").into_os_string())
            })
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| super::super::paths::home().join("AppData/Local/crush"))
            .join("crush.json")
    } else {
        std::env::var_os("CRUSH_GLOBAL_DATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("XDG_DATA_HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| super::super::paths::home().join(".local/share"))
                    .join("crush")
            })
            .join("crush.json")
    }
}

pub fn read_effective(path: &Path, value: &Value) -> Result<Vec<Profile>, String> {
    let data = data_path();
    if data == path {
        return read(value);
    }
    let bytes = files::read(&data)?;
    let text = files::text(bytes.as_deref())?;
    let effective = merge(Some(value), &files::parse(ConfigFormat::Json, &text)?);
    read(&effective)
}

pub fn write(
    path: &Path,
    value: &mut Value,
    profile: &Profile,
    operation: &str,
) -> Result<Vec<Change>, String> {
    let id = native_id(profile);
    let mut provider = native_entry(
        value.get("providers").and_then(|v| v.get(&id)),
        &profile.native.entry,
    )?;
    set(&mut provider, &["name"], json!(profile.name))?;
    set(
        &mut provider,
        &["type"],
        json!(match profile.protocol.as_str() {
            "anthropic" => "anthropic",
            "responses" => "openai",
            _ => "openai-compat",
        }),
    )?;
    set(&mut provider, &["base_url"], json!(profile.base_url))?;
    super::apply_credential(&mut provider, &["api_key"], profile)?;
    let mut models = provider
        .get("models")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for model in &profile.models {
        let index = models
            .iter()
            .position(|v| v.get("id").and_then(Value::as_str) == Some(&model.id));
        let mut item = index.map(|i| models[i].clone()).unwrap_or_else(|| json!({ "cost_per_1m_in": 0, "cost_per_1m_out": 0, "cost_per_1m_in_cached": 0, "cost_per_1m_out_cached": 0 }));
        set(&mut item, &["id"], json!(model.id))?;
        set(
            &mut item,
            &["name"],
            json!(if model.name.is_empty() {
                &model.id
            } else {
                &model.name
            }),
        )?;
        if let Some(v) = model.context_window {
            set(&mut item, &["context_window"], json!(v))?;
        }
        if let Some(v) = model.max_output_tokens {
            set(&mut item, &["default_max_tokens"], json!(v))?;
        }
        if let Some(v) = model.supports_images {
            set(&mut item, &["supports_attachments"], json!(v))?;
        }
        if let Some(v) = model.supports_reasoning {
            set(&mut item, &["can_reason"], json!(v))?;
        }
        if !model.reasoning_levels.is_empty() {
            set(
                &mut item,
                &["reasoning_levels"],
                json!(model.reasoning_levels),
            )?;
        }
        if model.effort.is_empty() || model.effort == "default" {
            super::remove(&mut item, &["default_reasoning_effort"]);
        } else {
            set(
                &mut item,
                &["default_reasoning_effort"],
                json!(model.effort),
            )?;
        }
        if let Some(i) = index {
            models[i] = item;
        } else {
            models.push(item);
        }
    }
    set(&mut provider, &["models"], json!(models))?;
    set(value, &["providers", &id], provider)?;
    let mut extra = Vec::new();
    if operation == "default" {
        let select = |state: &mut Value| -> Result<(), String> {
            set(state, &["models", "large", "provider"], json!(id))?;
            set(state, &["models", "large", "model"], json!(profile.model))?;
            if profile.effort.is_empty() || profile.effort == "default" {
                super::remove(state, &["models", "large", "reasoning_effort"]);
            } else {
                set(
                    state,
                    &["models", "large", "reasoning_effort"],
                    json!(profile.effort),
                )?;
            }
            Ok(())
        };
        let data = data_path();
        if data == path {
            select(value)?;
        } else {
            let mut change = Change::load(data)?;
            let original = files::text(change.before.as_deref())?;
            let mut state = files::parse(ConfigFormat::Json, &original)?;
            select(&mut state)?;
            change.after = Some(files::render(ConfigFormat::Json, &original, &state)?);
            extra.push(change);
        }
    }
    Ok(extra)
}
