use super::super::{
    files::{self, Change},
    Profile,
};
use super::{remove, set};
use serde_json::{json, Value};
use std::path::Path;

pub fn read(path: &Path, value: &Value) -> Result<Vec<Profile>, String> {
    if !value.is_object() {
        return Err("Gemini settings.json 必须是对象".into());
    }
    let env = files::read(&path.with_file_name(".env"))?;
    let text = files::text(env.as_deref())?;
    let mut profile = Profile::new(
        "gemini-current",
        "Gemini CLI 当前配置",
        "gemini",
        &env_value(&text, "GOOGLE_GEMINI_BASE_URL").unwrap_or_default(),
    );
    profile.model = value
        .pointer("/model/name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .into();
    if profile.model.is_empty() {
        profile.model = env_value(&text, "GEMINI_MODEL").unwrap_or_default();
    }
    profile.api_key = env_value(&text, "GEMINI_API_KEY").unwrap_or_default();
    if profile.base_url.is_empty() && profile.model.is_empty() && profile.api_key.is_empty() { return Ok(Vec::new()); }
    profile.native.settings =
        json!({ "model": value.get("model"), "security": value.get("security") });
    profile.native.auth = value
        .pointer("/security/auth/selectedType")
        .cloned()
        .unwrap_or(Value::Null);
    Ok(vec![profile])
}

pub fn write(
    path: &Path,
    value: &mut Value,
    profile: &Profile,
    warnings: &mut Vec<String>,
) -> Result<Vec<Change>, String> {
    if profile.kind == "gateway" {
        return Err("本网关尚无 Gemini 原生入口，不能接入本网关".into());
    }
    let mut change = Change::load(path.with_file_name(".env"))?;
    let original = files::text(change.before.as_deref())?;
    let mut updates = vec![
        ("GOOGLE_GEMINI_BASE_URL", Some(profile.base_url.clone())),
        ("GEMINI_MODEL", None),
    ];
    let key = super::super::validate::credential(profile)?;
    if let Some(key) = key {
        updates.push(("GEMINI_API_KEY", Some(key)));
    }
    if profile.auth_mode == "preserve" && profile.base_url.is_empty() {
        updates[0].1 = None;
        if let Some(auth) = profile.native.auth.as_str() {
            set(value, &["security", "auth", "selectedType"], json!(auth))?;
        }
    } else {
        set(
            value,
            &["security", "auth", "selectedType"],
            json!("gemini-api-key"),
        )?;
    }
    if profile.model.is_empty() {
        remove(value, &["model", "name"]);
    } else {
        set(value, &["model", "name"], json!(profile.model))?;
    }
    let rendered = merge_env(&original, &updates)?;
    change.after = Some(rendered.into_bytes());
    warnings.push(
        "同时修改 settings.json 与 .env；已有 shell 环境变量或项目 .env 可能覆盖此配置。".into(),
    );
    Ok(vec![change])
}

pub fn env_value(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim().strip_prefix("export ").unwrap_or(line.trim());
        if line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim() != key {
            continue;
        }
        let raw = value.trim();
        if raw.starts_with('"') {
            return serde_json::from_str::<String>(raw).ok();
        }
        if raw.starts_with('\'') && raw.ends_with('\'') {
            return Some(raw[1..raw.len() - 1].into());
        }
        return Some(raw.split(" #").next().unwrap_or("").trim().into());
    }
    None
}

pub(super) fn merge_env(original: &str, updates: &[(&str, Option<String>)]) -> Result<String, String> {
    let mut lines = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in original.lines() {
        let trimmed = line.trim();
        let assignment = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        let key = assignment.split_once('=').map(|(name, _)| name.trim());
        if let Some((name, value)) = updates
            .iter()
            .find(|(name, _)| Some(*name) == key && !trimmed.starts_with('#'))
        {
            if seen.insert(*name) {
                if let Some(value) = value {
                    lines.push(format!(
                        "{}{}={}",
                        if trimmed.starts_with("export ") {
                            "export "
                        } else {
                            ""
                        },
                        name,
                        serde_json::to_string(value)
                            .map_err(|e| format!("生成环境变量失败: {e}"))?
                    ));
                }
            }
        } else {
            lines.push(line.into());
        }
    }
    for (key, value) in updates {
        if !seen.contains(key) {
            if let Some(value) = value {
                lines.push(format!(
                    "{key}={}",
                    serde_json::to_string(value).map_err(|e| format!("生成环境变量失败: {e}"))?
                ));
            }
        }
    }
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    Ok(lines.join(newline) + newline)
}
