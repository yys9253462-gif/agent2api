//! 出站信封已在插件中转换；在响应缓存所在进程按调用 id 回填真实签名。

use serde_json::Value;

use super::super::protocol::antigravity_signature_cache;
use crate::server::logging;

pub(super) fn fill_cached(body: &mut Value) {
    let model = body.get("model").and_then(Value::as_str).unwrap_or("").to_ascii_lowercase();
    if !model.starts_with("gemini-") && !model.starts_with("gpt-oss-") { return }
    let session = body.pointer("/request/sessionId").and_then(Value::as_str).unwrap_or("").to_string();
    let Some(contents) = body.pointer_mut("/request/contents").and_then(Value::as_array_mut) else { return };
    let mut filled = 0;
    for turn in contents.iter_mut().filter(|turn| turn.get("role").and_then(Value::as_str) == Some("model")) {
        let Some(parts) = turn.get_mut("parts").and_then(Value::as_array_mut) else { continue };
        for part in parts.iter_mut().filter_map(Value::as_object_mut) {
            let current = part.get("thoughtSignature").and_then(Value::as_str).unwrap_or("").trim();
            if !current.is_empty() && current != "skip_thought_signature_validator"
                && current != "context_engineering_is_the_way_to_go"
            { continue }
            let id = part.get("functionCall").and_then(|call| call.get("id"))
                .and_then(Value::as_str).unwrap_or("");
            // 函数名可在同一历史里重复，不能借用最近一次同名调用的签名。
            if let Some(signature) = antigravity_signature_cache::lookup_call(&session, id) {
                part.insert("thoughtSignature".to_string(), Value::String(signature));
                filled += 1;
            }
        }
    }
    if filled > 0 {
        logging::verbose("[Antigravity]", &format!("已从响应缓存回填 {filled} 处历史工具调用的真实思考签名"));
    }
}
