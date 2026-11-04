use super::super::Profile;
use super::{merge, native_entry, native_id, remove, set, string};
use serde_json::{json, Value};

pub fn read(value: &Value) -> Result<Vec<Profile>, String> {
    if !value.is_object() {
        return Err("Codex config.toml 必须是表".into());
    }
    let profile_key = string(value, "profile");
    let runtime = value
        .get("profiles")
        .and_then(|profiles| profiles.get(&profile_key));
    let effective = merge(Some(value), runtime.unwrap_or(&Value::Null));
    let mut active = string(&effective, "model_provider");
    if active.is_empty() {
        active = "openai".into();
    }
    let providers = value.get("model_providers").and_then(Value::as_object);
    let mut profiles = Vec::new();
    if let Some(providers) = providers {
        for (id, entry) in providers {
            let base = string(entry, "base_url");
            if base.is_empty() {
                continue;
            }
            let name = string(entry, "name");
            let mut profile = Profile::new(id, if name.is_empty() { id } else { &name }, "responses", &base);
            let wire = string(entry, "wire_api");
            if !wire.is_empty() && wire != "responses" {
                profile.protocol = wire;
            }
            profile.enabled = id == &active;
            profile.default = profile.enabled;
            profile.model = if profile.enabled {
                string(&effective, "model")
            } else {
                String::new()
            };
            profile.effort = if profile.enabled {
                string(&effective, "model_reasoning_effort")
            } else {
                String::new()
            };
            profile.env_key = string(entry, "env_key");
            profile.api_key = string(entry, "experimental_bearer_token");
            profile.native.entry = entry.clone();
            profile.native.settings = json!({ "profile": profile_key, "model_provider": id, "model": profile.model, "model_reasoning_effort": profile.effort,
                "runtime": { "model_provider": runtime.unwrap_or(value).get("model_provider"), "model": runtime.unwrap_or(value).get("model"), "model_reasoning_effort": runtime.unwrap_or(value).get("model_reasoning_effort") },
                "rootEffort": value.get("model_reasoning_effort") });
            if !profile.env_key.is_empty() {
                profile.auth_mode = "env".into();
            }
            profiles.push(profile);
        }
    }
    if !profiles.iter().any(|p| p.enabled) && ["model_provider", "model", "model_reasoning_effort", "openai_base_url"].iter().any(|key| effective.get(*key).is_some()) {
        let base = string(&effective, "openai_base_url");
        let mut profile = Profile::new(&active, "Codex 官方或内置配置", "responses", &base);
        profile.model = string(&effective, "model");
        profile.effort = string(&effective, "model_reasoning_effort");
        profile.native.settings = json!({ "profile": profile_key, "model_provider": active, "model": profile.model, "model_reasoning_effort": profile.effort });
        profiles.push(profile);
    }
    Ok(profiles)
}

pub fn write(
    value: &mut Value,
    profile: &Profile,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    let target = native_id(profile);
    let mut entry = native_entry(
        value.get("model_providers").and_then(|v| v.get(&target)),
        &profile.native.entry,
    )?;
    let official = profile.base_url.is_empty() && profile.auth_mode == "preserve";
    if !official {
        set(&mut entry, &["name"], json!(profile.name))?;
        set(&mut entry, &["base_url"], json!(profile.base_url))?;
        set(&mut entry, &["wire_api"], json!("responses"))?;
        match profile.auth_mode.as_str() {
            "env" => {
                set(&mut entry, &["env_key"], json!(profile.env_key))?;
                remove(&mut entry, &["experimental_bearer_token"]);
                set(&mut entry, &["requires_openai_auth"], json!(false))?;
                warnings.push(format!(
                    "请确认启动 Codex 的环境中已设置 {}；本页不会修改系统环境变量。",
                    profile.env_key
                ));
            }
            "gateway" | "key" | "none" => {
                remove(&mut entry, &["env_key"]);
                set(&mut entry, &["requires_openai_auth"], json!(false))?;
                if let Some(key) = super::super::validate::credential(profile)? {
                    set(&mut entry, &["experimental_bearer_token"], json!(key))?;
                }
            }
            _ => {
                if let Some(key) = super::super::validate::credential(profile)? {
                    set(&mut entry, &["experimental_bearer_token"], json!(key))?;
                }
            }
        }
        set(value, &["model_providers", &target], entry)?;
    }
    let selected = string(value, "profile");
    let path: Vec<&str> = if selected.is_empty() {
        Vec::new()
    } else {
        vec!["profiles", &selected]
    };
    let set_runtime = |value: &mut Value, key: &str, item: Value| {
        let mut keys = path.clone();
        keys.push(key);
        set(value, &keys, item)
    };
    set_runtime(value, "model_provider", json!(target))?;
    if !profile.model.is_empty() {
        set_runtime(value, "model", json!(profile.model))?;
    }
    if profile.effort.is_empty() || profile.effort == "default" {
        let mut keys = path.clone();
        keys.push("model_reasoning_effort");
        remove(value, &keys);
        if !selected.is_empty() {
            remove(value, &["model_reasoning_effort"]);
        }
    } else {
        set_runtime(value, "model_reasoning_effort", json!(profile.effort))?;
    }
    if !selected.is_empty() {
        warnings.push(format!(
            "当前使用 Codex profile「{selected}」；模型选择写入此 profile。"
        ));
    }
    Ok(())
}
