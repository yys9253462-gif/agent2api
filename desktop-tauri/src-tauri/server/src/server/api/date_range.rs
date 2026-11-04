use std::collections::HashMap;

use chrono::NaiveDate;

pub(super) fn parse_day_param(params: &HashMap<String, String>, key: &str) -> Result<NaiveDate, String> {
    let text = params.get(key).map(|value| value.trim()).filter(|value| !value.is_empty())
        .ok_or_else(|| format!("自定义日期范围必须带 {key}（格式 YYYY-MM-DD）"))?;
    let valid = text.len() == 10 && text.bytes().enumerate().all(|(index, byte)| {
        if index == 4 || index == 7 { byte == b'-' } else { byte.is_ascii_digit() }
    });
    if !valid {
        return Err(format!("{key} 格式非法: {text}（需要 YYYY-MM-DD）"));
    }
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .map_err(|_| format!("{key} 格式非法: {text}（需要 YYYY-MM-DD）"))
}

pub(super) fn date_bounds(params: &HashMap<String, String>) -> Result<Option<(i64, i64)>, String> {
    if !params.contains_key("startDate") && !params.contains_key("endDate") {
        return Ok(None);
    }
    let start = parse_day_param(params, "startDate")?;
    let end = parse_day_param(params, "endDate")?;
    if start > end { return Err("开始日期不能晚于结束日期".to_string()); }
    crate::server::request_stats::request_date_bounds(start, end).map(Some)
        .ok_or_else(|| "日期范围超出可处理范围".to_string())
}
