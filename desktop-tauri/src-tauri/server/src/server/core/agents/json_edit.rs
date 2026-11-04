use jsonc_parser::{
    cst::{CstArray, CstInputValue, CstNode, CstObject, CstRootNode},
    ParseOptions,
};
use serde_json::Value;

pub fn parse(text: &str, comments: bool) -> Result<Value, String> {
    let text = text.trim_start_matches('\u{feff}');
    if text.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    if !comments {
        return serde_json::from_str(text).map_err(|e| format!("配置不是合法 JSON: {e}"));
    }
    let root =
        CstRootNode::parse(text, &options(true)).map_err(|e| format!("配置不是合法 JSONC: {e}"))?;
    root.value()
        .and_then(|node| node.to_serde_value())
        .ok_or_else(|| "配置文件为空或无法读取".into())
}

fn options(comments: bool) -> ParseOptions {
    ParseOptions {
        allow_comments: comments,
        allow_trailing_commas: comments,
        allow_loose_object_property_names: false,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
    }
}

pub fn render(original: &str, desired: &Value, comments: bool) -> Result<String, String> {
    let bom = original.starts_with('\u{feff}');
    let text = original.trim_start_matches('\u{feff}');
    let before = parse(text, comments)?;
    if before == *desired && !text.trim().is_empty() {
        return Ok(original.into());
    }
    let source = if text.trim().is_empty() { "{}\n" } else { text };
    let root =
        CstRootNode::parse(source, &options(comments)).map_err(|e| format!("配置编辑失败: {e}"))?;
    let node = root.value().ok_or("配置没有根节点")?;
    if let (Some(object), Some(before), Some(after)) =
        (node.as_object(), before.as_object(), desired.as_object())
    {
        patch_object(object, before, after)?;
    } else if let (Some(array), Some(before), Some(after)) =
        (node.as_array(), before.as_array(), desired.as_array())
    {
        patch_array(array, before, after)?;
    } else {
        return Err("配置根结构与客户端要求不一致，拒绝覆盖".into());
    }
    let mut rendered = root.to_string();
    if text.ends_with('\n') && !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    if bom {
        rendered.insert(0, '\u{feff}');
    }
    if parse(&rendered, comments)? != *desired {
        return Err("配置编辑结果与预期不一致，已停止写入".into());
    }
    Ok(rendered)
}

fn patch_object(
    object: CstObject,
    before: &serde_json::Map<String, Value>,
    after: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    for key in before.keys().filter(|key| !after.contains_key(*key)) {
        if let Some(property) = object.get(key) {
            property.remove();
        }
    }
    for (key, value) in after {
        if before.get(key) == Some(value) {
            continue;
        }
        if let Some(property) = object.get(key) {
            if let (Some(child), Some(old), Some(new)) = (
                property.object_value(),
                before.get(key).and_then(Value::as_object),
                value.as_object(),
            ) {
                patch_object(child, old, new)?;
            } else if let (Some(child), Some(old), Some(new)) = (
                property.value().and_then(|n| n.as_array()),
                before.get(key).and_then(Value::as_array),
                value.as_array(),
            ) {
                patch_array(child, old, new)?;
            } else {
                property.set_value(input(value));
            }
        } else {
            object.append(key, input(value));
        }
    }
    Ok(())
}

fn identity(value: &Value) -> Option<String> {
    if let (Some(provider), Some(model)) = (
        value.get("providerId").and_then(Value::as_str),
        value.get("modelId").and_then(Value::as_str),
    ) {
        return serde_json::to_string(&(provider, model)).ok();
    }
    value
        .get("providerId")
        .or_else(|| value.get("id"))
        .or_else(|| value.get("modelId"))
        .or_else(|| value.get("name"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn patch_array(array: CstArray, before: &[Value], after: &[Value]) -> Result<(), String> {
    let nodes = array.elements();
    if nodes.len() != before.len() {
        return Err("数组元素无法精确匹配，已停止写入".into());
    }
    let keyed = before.iter().all(|value| identity(value).is_some())
        && after.iter().all(|value| identity(value).is_some());
    if keyed {
        let old_ids: Vec<_> = before.iter().filter_map(identity).collect();
        let new_ids: Vec<_> = after.iter().filter_map(identity).collect();
        let kept_old: Vec<_> = old_ids
            .iter()
            .filter(|id| new_ids.contains(id))
            .cloned()
            .collect();
        let kept_new: Vec<_> = new_ids
            .iter()
            .filter(|id| old_ids.contains(id))
            .cloned()
            .collect();
        // 调整已有顺序会改变注释归属；仅追加或删除时逐条保留原节点。
        if kept_old == kept_new
            && after
                .iter()
                .skip(kept_new.len())
                .all(|v| !old_ids.contains(&identity(v).unwrap_or_default()))
        {
            for (old, node) in before.iter().zip(nodes) {
                if let Some(new) = after.iter().find(|new| identity(new) == identity(old)) {
                    patch_node(node, old, new)?;
                } else {
                    node.remove();
                }
            }
            for value in after
                .iter()
                .filter(|value| !old_ids.contains(&identity(value).unwrap_or_default()))
            {
                array.append(input(value));
            }
            return Ok(());
        }
    }
    if before.len() == after.len() {
        for ((old, new), node) in before.iter().zip(after).zip(nodes) {
            patch_node(node, old, new)?;
        }
    } else {
        array.replace_with(input(&Value::Array(after.to_vec())));
    }
    Ok(())
}

fn patch_node(node: CstNode, before: &Value, after: &Value) -> Result<(), String> {
    if before == after {
        return Ok(());
    }
    if let (Some(object), Some(before), Some(after)) =
        (node.as_object(), before.as_object(), after.as_object())
    {
        return patch_object(object, before, after);
    }
    if let (Some(array), Some(before), Some(after)) =
        (node.as_array(), before.as_array(), after.as_array())
    {
        return patch_array(array, before, after);
    }
    match node {
        CstNode::Container(jsonc_parser::cst::CstContainerNode::Object(node)) => {
            node.replace_with(input(after));
        }
        CstNode::Container(jsonc_parser::cst::CstContainerNode::Array(node)) => {
            node.replace_with(input(after));
        }
        CstNode::Leaf(jsonc_parser::cst::CstLeafNode::StringLit(node)) => {
            node.replace_with(input(after));
        }
        CstNode::Leaf(jsonc_parser::cst::CstLeafNode::NumberLit(node)) => {
            node.replace_with(input(after));
        }
        CstNode::Leaf(jsonc_parser::cst::CstLeafNode::BooleanLit(node)) => {
            node.replace_with(input(after));
        }
        CstNode::Leaf(jsonc_parser::cst::CstLeafNode::NullKeyword(node)) => {
            node.replace_with(input(after));
        }
        _ => return Err("无法原位修改配置节点".into()),
    }
    Ok(())
}

fn input(value: &Value) -> CstInputValue {
    match value {
        Value::Null => CstInputValue::Null,
        Value::Bool(value) => CstInputValue::Bool(*value),
        Value::Number(value) => CstInputValue::Number(value.to_string()),
        Value::String(value) => CstInputValue::String(value.clone()),
        Value::Array(values) => CstInputValue::Array(values.iter().map(input).collect()),
        Value::Object(values) => CstInputValue::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), input(value)))
                .collect(),
        ),
    }
}
