use super::super::{types::ClaudeModels, Profile};
use super::{remove, set, string};
use serde_json::{json, Value};

const MODEL_KEYS: &[&str] = &[
    "ANTHROPIC_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "CLAUDE_CODE_SUBAGENT_MODEL",
];

pub fn read(value: &Value) -> Result<Vec<Profile>, String> {
    if !value.is_object() {
        return Err("Claude Code settings.json 必须是对象".into());
    }
    let env = value.get("env").cloned().unwrap_or_else(|| json!({}));
    if !env.is_object() {
        return Err("Claude Code env 必须是对象".into());
    }
    let mut profile = Profile::new(
        "claude-current",
        "Claude Code 当前配置",
        "anthropic",
        &string(&env, "ANTHROPIC_BASE_URL"),
    );
    profile.model = string(&env, "ANTHROPIC_MODEL");
    if profile.model.is_empty() {
        profile.model = string(value, "model");
    }
    profile.effort = string(&env, "CLAUDE_CODE_EFFORT_LEVEL");
    if profile.effort.is_empty() {
        profile.effort = string(value, "effortLevel");
    }
    profile.api_key = string(&env, "ANTHROPIC_AUTH_TOKEN");
    if profile.api_key.is_empty() {
        profile.api_key = string(&env, "ANTHROPIC_API_KEY");
    }
    profile.claude_models = ClaudeModels {
        opus: string(&env, "ANTHROPIC_DEFAULT_OPUS_MODEL"),
        sonnet: string(&env, "ANTHROPIC_DEFAULT_SONNET_MODEL"),
        haiku: string(&env, "ANTHROPIC_DEFAULT_HAIKU_MODEL"),
        subagent: string(&env, "CLAUDE_CODE_SUBAGENT_MODEL"),
    };
    let mut settings = json!({ "env": {} });
    for key in MODEL_KEYS.iter().copied().chain([
        "ANTHROPIC_BASE_URL",
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_API_KEY",
        "CLAUDE_CODE_EFFORT_LEVEL",
    ]) {
        if let Some(item) = env.get(key) {
            set(&mut settings, &["env", key], item.clone())?;
        }
    }
    for key in ["model", "effortLevel"] {
        if let Some(item) = value.get(key) {
            set(&mut settings, &[key], item.clone())?;
        }
    }
    if settings.get("env").and_then(Value::as_object).is_some_and(|env| env.is_empty())
        && settings.get("model").is_none() && settings.get("effortLevel").is_none()
    { return Ok(Vec::new()); }
    profile.native.settings = settings;
    Ok(vec![profile])
}

pub fn write(
    value: &mut Value,
    profile: &Profile,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    if profile.base_url.is_empty() && profile.auth_mode == "preserve" {
        remove(value, &["env", "ANTHROPIC_BASE_URL"]);
    } else {
        set(
            value,
            &["env", "ANTHROPIC_BASE_URL"],
            json!(profile.base_url),
        )?;
    }
    if profile.auth_mode == "preserve" && !profile.native.settings.is_null() {
        for key in ["ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_API_KEY"] {
            if let Some(item) = profile.native.settings.pointer(&format!("/env/{key}")) {
                set(value, &["env", key], item.clone())?;
            } else {
                remove(value, &["env", key]);
            }
        }
    } else if let Some(key) = super::super::validate::credential(profile)? {
        let field = if profile.auth_mode == "key" {
            "ANTHROPIC_API_KEY"
        } else {
            "ANTHROPIC_AUTH_TOKEN"
        };
        let other = if field == "ANTHROPIC_API_KEY" {
            "ANTHROPIC_AUTH_TOKEN"
        } else {
            "ANTHROPIC_API_KEY"
        };
        set(value, &["env", field], json!(key))?;
        set(value, &["env", other], json!(""))?;
    }
    for (key, model) in [
        ("ANTHROPIC_MODEL", &profile.model),
        ("ANTHROPIC_DEFAULT_OPUS_MODEL", &profile.claude_models.opus),
        (
            "ANTHROPIC_DEFAULT_SONNET_MODEL",
            &profile.claude_models.sonnet,
        ),
        (
            "ANTHROPIC_DEFAULT_HAIKU_MODEL",
            &profile.claude_models.haiku,
        ),
        (
            "CLAUDE_CODE_SUBAGENT_MODEL",
            &profile.claude_models.subagent,
        ),
    ] {
        if model.is_empty() {
            remove(value, &["env", key]);
        } else {
            set(value, &["env", key], json!(model))?;
        }
        let display_key = format!("{key}_NAME");
        if value.pointer(&format!("/env/{display_key}")).is_some() {
            remove(value, &["env", &display_key]);
        }
    }
    if profile.model.is_empty() {
        remove(value, &["model"]);
    } else {
        set(value, &["model"], json!(profile.model))?;
    }
    if profile.effort.is_empty() || profile.effort == "default" {
        remove(value, &["env", "CLAUDE_CODE_EFFORT_LEVEL"]);
        remove(value, &["effortLevel"]);
    } else {
        set(
            value,
            &["env", "CLAUDE_CODE_EFFORT_LEVEL"],
            json!(profile.effort),
        )?;
        if profile.effort == "auto" {
            remove(value, &["effortLevel"]);
        } else {
            set(value, &["effortLevel"], json!(profile.effort))?;
        }
    }
    warnings.push("保留权限、Hooks、MCP 等设置；模型别名映射只修改填写的原生模型字段。".into());
    Ok(())
}
