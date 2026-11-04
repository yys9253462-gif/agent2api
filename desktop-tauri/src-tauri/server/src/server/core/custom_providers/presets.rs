use serde_json::{Map, Value};

const MAX_KEY_LENGTH: usize = 64;

pub(super) fn validate(value: Option<&Value>) -> Result<String, String> {
    let key = match value {
        None | Some(Value::Null) => return Ok(String::new()),
        Some(Value::String(key)) => key.trim(),
        Some(_) => return Err("presetKey 必须是字符串".to_string()),
    };
    if key.is_empty() || valid_key(key) {
        Ok(key.to_string())
    } else {
        Err("presetKey 最多 64 个字符，只能包含小写字母、数字、连字符和下划线".to_string())
    }
}

pub(super) fn key_of_record(object: &Map<String, Value>) -> String {
    if object.contains_key("presetKey") {
        return validate(object.get("presetKey")).unwrap_or_default();
    }
    // 旧记录只按显式保存的适配方案补标识，不根据展示名或自建站点域名猜模板。
    let key = match object.get("balanceScheme").and_then(Value::as_str) {
        Some("new_api") => "newapi",
        Some("sub2api") => "sub2api",
        Some("deepseek") => "deepseek",
        Some("opencode")
            if object.get("clientEmulation").and_then(Value::as_str) != Some("opencode") =>
        {
            "opencode-go"
        }
        Some("sensenova") => "sensenova",
        Some("zhipu") => "glm-cn",
        Some("zhipu_zai") => "glm",
        _ if object.get("clientEmulation").and_then(Value::as_str) == Some("opencode") => {
            "opencode-zen"
        }
        _ => "",
    };
    key.to_string()
}

pub(super) fn write_key(item: &mut Value, key: String) {
    if !key.is_empty() {
        if let Some(object) = item.as_object_mut() {
            object.insert("presetKey".to_string(), Value::String(key));
        }
    }
}

fn valid_key(key: &str) -> bool {
    key.len() <= MAX_KEY_LENGTH
        && key.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}
