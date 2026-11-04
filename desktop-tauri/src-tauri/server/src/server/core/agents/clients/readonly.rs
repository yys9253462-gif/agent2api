use super::super::{ClientDef, Profile};
use super::string;
use serde_json::{json, Value};

pub fn read(def: &ClientDef, value: &Value) -> Result<Vec<Profile>, String> {
    if !value.is_object() {
        return Err("客户端配置根必须是对象".into());
    }
    if def.id == "cursor-cli" {
        let id = value
            .pointer("/model/modelId")
            .and_then(Value::as_str)
            .unwrap_or("");
        if id.is_empty() {
            return Ok(Vec::new());
        }
        let name = value
            .pointer("/model/displayName")
            .and_then(Value::as_str)
            .unwrap_or("Cursor 当前模型");
        let mut profile = Profile::new("cursor-current", name, "native", "");
        profile.model = id.into();
        profile.native.settings = json!({ "model": value.get("model") });
        return Ok(vec![profile]);
    }
    let mut out = Vec::new();
    if let Some(providers) = value.get("providers").and_then(Value::as_object) {
        for (id, provider) in providers {
            let base = string(provider, "baseUrl");
            let mut profile = Profile::new(
                id,
                &string(provider, "displayName"),
                &string(provider, "protocol"),
                &base,
            );
            profile.api_key = string(provider, "apiKey");
            profile.model = string(provider, "model");
            profile.enabled = value
                .pointer("/model/name")
                .and_then(Value::as_str)
                .is_some_and(|m| m.starts_with(&format!("{id}/")));
            profile.default = profile.enabled;
            profile.native.entry = provider.clone();
            out.push(profile);
        }
    }
    Ok(out)
}
