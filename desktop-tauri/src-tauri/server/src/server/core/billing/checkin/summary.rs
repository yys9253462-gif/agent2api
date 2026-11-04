use serde_json::Value;

use super::{claim_pending, day_completed};

fn text(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

pub(crate) fn failure_of(row: &Value) -> Option<String> {
    let claim = row.get("claim");
    let error = text(row.get("error"));
    if error.is_none()
        && (day_completed(row) || claim.is_some_and(claim_pending))
    {
        return None;
    }
    let message = error
        .or_else(|| text(claim.and_then(|claim| claim.get("error"))))
        .or_else(|| text(claim.and_then(|claim| claim.get("msg"))))
        .unwrap_or("签到未完成");
    let name = text(row.get("name"))
        .or_else(|| text(row.get("id")))
        .unwrap_or("未知账号");
    Some(format!("{name}（{message}）"))
}
