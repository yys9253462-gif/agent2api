use serde_json::{json, Value};
use super::super::{files, ClientDef, ConfigFormat};
use super::paths::ResourcePaths;

pub fn read(paths: &ResourcePaths) -> Result<Value, String> {
    let (path, format, parents) = paths.mcp.as_ref().ok_or("尚未支持这个客户端的 MCP 配置")?;
    let bytes = files::read(path)?;
    let original = files::text(bytes.as_deref())?;
    let value = files::parse(*format, &original)?;
    let mut node = &value;
    for key in parents {
        match node.get(*key) {
            Some(child) if child.is_object() => node = child,
            Some(_) => return Err(format!("MCP 配置中的 {key} 不是对象，拒绝覆盖")),
            None => return Ok(json!({ "path": path, "exists": bytes.is_some(), "servers": [], "revision": super::revision(bytes.as_deref()) })),
        }
    }
    let servers = node.as_object().ok_or("MCP 配置根不是对象")?.iter().map(|(name, entry)| {
        let endpoint = entry.get("url").or_else(|| entry.get("httpUrl")).and_then(Value::as_str).map(str::to_string)
            .unwrap_or_else(|| match entry.get("command") {
                Some(Value::String(value)) => value.clone(),
                Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" "),
                _ => String::new(),
            });
        json!({ "name": name, "endpoint": endpoint, "config": entry })
    }).collect::<Vec<_>>();
    Ok(json!({ "path": path, "exists": bytes.is_some(), "servers": servers, "revision": super::revision(bytes.as_deref()) }))
}

pub fn change(def: &ClientDef, paths: &ResourcePaths, name: &str, config: Option<&Value>) -> Result<files::Change, String> {
    if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) { return Err("MCP 服务名称无效".into()); }
    let (path, format, parents) = paths.mcp.as_ref().ok_or("尚未支持这个客户端的 MCP 配置")?;
    if let Some(config) = config { validate(def, config)?; }
    let mut change = files::Change::load(path.clone())?;
    super::reject_links(path)?;
    let original = files::text(change.before.as_deref())?;
    let mut value = files::parse(*format, &original)?;
    let mut node = &mut value;
    for key in parents {
        let object = node.as_object_mut().ok_or("MCP 配置层级不是对象，拒绝覆盖")?;
        node = object.entry(*key).or_insert_with(|| json!({}));
        if !node.is_object() { return Err(format!("MCP 配置中的 {key} 不是对象，拒绝覆盖")); }
    }
    let servers = node.as_object_mut().ok_or("MCP 服务配置不是对象")?;
    if let Some(config) = config { servers.insert(name.to_string(), config.clone()); }
    else if servers.remove(name).is_none() { return Err("MCP 服务不存在".into()); }
    change.after = Some(files::render(*format, &original, &value)?);
    Ok(change)
}

fn validate(def: &ClientDef, value: &Value) -> Result<(), String> {
    let object = value.as_object().ok_or("MCP 配置必须是 JSON 对象")?;
    if value.to_string().len() > 256 * 1024 { return Err("MCP 服务配置过大".into()); }
    if def.id == "opencode" {
        match object.get("type").and_then(Value::as_str) {
            Some("local") => {
                if !object.get("command").and_then(Value::as_array).is_some_and(|items| !items.is_empty() && items.iter().all(|item| item.as_str().is_some_and(|text| !text.is_empty()))) {
                    return Err("OpenCode 本地 MCP 的 command 必须是非空字符串数组".into());
                }
            }
            Some("remote") => validate_url(object.get("url"))?,
            _ => return Err("OpenCode MCP 类型必须为 local 或 remote".into()),
        }
    } else if let Some(url) = object.get("httpUrl").or_else(|| object.get("url")) { validate_url(Some(url))?; }
    else if !object.get("command").and_then(Value::as_str).is_some_and(|text| !text.trim().is_empty()) {
        return Err("MCP 配置需要 command 或 HTTP(S) 地址".into());
    }
    if let Some(args) = object.get("args") {
        if !args.as_array().is_some_and(|items| items.iter().all(Value::is_string)) { return Err("MCP args 必须是字符串数组".into()); }
    }
    if def.id == "gemini-cli" && object.get("type").and_then(Value::as_str) == Some("http") && !object.contains_key("httpUrl") {
        return Err("Gemini CLI 的 HTTP MCP 请使用 httpUrl 字段".into());
    }
    if *format_for(def) == ConfigFormat::Toml && object.values().any(Value::is_null) { return Err("TOML MCP 不支持 null 字段".into()); }
    Ok(())
}

fn format_for(def: &ClientDef) -> &ConfigFormat { &def.format }
fn validate_url(value: Option<&Value>) -> Result<(), String> {
    let valid = value.and_then(Value::as_str).and_then(|text| url::Url::parse(text).ok()).is_some_and(|url| matches!(url.scheme(), "https" | "http") && url.host_str().is_some() && url.username().is_empty() && url.password().is_none());
    if valid { Ok(()) } else { Err("MCP 地址必须是有效的 HTTP(S) URL".into()) }
}
