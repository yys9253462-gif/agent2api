use serde_json::{Map, Value};

use super::legacy::{self, LimiterAction, LimiterKind, LimiterReset};
use super::types::*;

pub fn effective_groups_in(fields: &Map<String, Value>) -> Vec<Group> {
    if let Some(value) = fields.get("limiterGroups") {
        return normalize_groups(value).unwrap_or_default();
    }
    groups_from_rules(&legacy::effective_rules_in(fields))
}

pub fn groups_from_rules(rules: &[legacy::LimiterRule]) -> Vec<Group> {
    rules.iter().map(|rule| {
        let token = rule.kind == LimiterKind::Token;
        Group {
            enabled: rule.enabled,
            action: if rule.action == LimiterAction::Disable { Action::Disable } else { Action::Skip },
            target: Target::Account,
            conditions: vec![Condition {
                join: Join::Or,
                source_id: String::new(),
                source: None,
                models: Vec::new(),
                metric: if token { Metric::Tokens } else { Metric::Remaining },
                comparison: if token { Comparison::Gte } else { Comparison::Lt },
                threshold: rule.threshold,
                window: token.then(|| Window {
                    reset: if rule.reset == LimiterReset::Daily { Reset::Daily } else { Reset::Fixed },
                    seconds: rule.period,
                    ..Window::default()
                }),
            }],
        }
    }).collect()
}

pub fn effective_groups(account: &Value) -> Vec<Group> {
    account.as_object().map(effective_groups_in).unwrap_or_default()
}

pub fn normalize_groups(value: &Value) -> Result<Vec<Group>, String> {
    let mut groups: Vec<Group> = serde_json::from_value(value.clone())
        .map_err(|_| "限制器必须是完整的条件组数组".to_string())?;
    if groups.len() > 64 { return Err("最多添加 64 个限制条件组".to_string()); }
    for group in &mut groups {
        if group.conditions.is_empty() || group.conditions.len() > 32 {
            return Err("每个条件组必须有 1~32 条条件".to_string());
        }
        for (index, condition) in group.conditions.iter_mut().enumerate() {
            if !condition.threshold.is_finite() || condition.threshold < 0.0 {
                return Err("限制阈值必须是非负有限数字".to_string());
            }
            if matches!(condition.metric, Metric::Tokens | Metric::Requests)
                && (condition.threshold <= 0.0 || condition.threshold.fract() != 0.0
                    || condition.threshold > 9_007_199_254_740_991.0) {
                return Err("本地消耗上限必须是大于 0 的安全整数".to_string());
            }
            if matches!(condition.metric, Metric::RemainingPercent | Metric::UsedPercent)
                && condition.threshold > 100.0 {
                return Err("百分比阈值必须在 0~100 之间".to_string());
            }
            condition.source_id = condition.source_id.trim().to_string();
            if condition.source_id.len() > 512 || condition.models.len() > 128 {
                return Err("额度来源或模型范围过大".to_string());
            }
            condition.models = condition.models.iter().map(|model| model.trim().to_string())
                .filter(|model| !model.is_empty()).collect();
            if condition.models.iter().any(|model| model.len() > 256) {
                return Err("模型标识过长".to_string());
            }
            condition.models.sort();
            condition.models.dedup();
            if index == 0 { condition.join = Join::Or; }
            if let Some(source) = &condition.source {
                if source.id != condition.source_id || source.label.len() > 512
                    || source.channel.len() > 128 || source.provider.len() > 128
                    || source.plan_id.len() > 512 || source.plan_name.len() > 512
                    || source.models.len() > 128 || source.models.iter().any(|model| model.len() > 256)
                    || source.starts_at < 0 || source.ends_at < 0 {
                    return Err("额度来源与所选来源不一致".to_string());
                }
                if let Some(window) = &source.window { validate_window(window)?; }
                if !source.models.is_empty() && condition.models.iter().any(|model| {
                    !source.models.iter().any(|wanted| wanted.eq_ignore_ascii_case(model))
                }) {
                    return Err("所选模型不属于该额度来源".to_string());
                }
                condition.source = Some(source.identity());
            }
            if matches!(condition.metric, Metric::Tokens | Metric::Requests) {
                let window = condition.window.as_ref().or_else(|| condition.source.as_ref().and_then(|source| source.window.as_ref()))
                    .ok_or_else(|| "本地消耗条件必须选择统计周期".to_string())?;
                validate_window(window)?;
            }
            if let Some(window) = &condition.window { validate_window(window)?; }
        }
    }
    Ok(groups)
}

fn validate_window(window: &Window) -> Result<(), String> {
    if matches!(window.reset, Reset::Fixed | Reset::Rolling)
        && !(60..=2_678_400).contains(&window.seconds) {
        return Err("统计窗口必须是 1 分钟~31 天".to_string());
    }
    if window.starts_at < 0 || window.ends_at < 0
        || (window.ends_at > 0 && window.ends_at <= window.starts_at) {
        return Err("统计结束时间必须晚于开始时间".to_string());
    }
    if window.utc_offset_minutes.is_some_and(|offset| !(-720..=840).contains(&offset)) {
        return Err("重置时区偏移无效".to_string());
    }
    Ok(())
}

pub fn legacy_rules(groups: &[Group]) -> Vec<legacy::LimiterRule> {
    groups.iter().filter_map(|group| {
        if group.target != Target::Account || group.conditions.len() != 1 { return None; }
        let c = group.conditions.first()?;
        if !c.source_id.is_empty() || !c.models.is_empty() || c.threshold <= 0.0 { return None; }
        let (kind, reset, period) = match c.metric {
            Metric::Remaining if c.comparison == Comparison::Lt => (LimiterKind::Balance, LimiterReset::Fixed, 0),
            Metric::Tokens if c.comparison == Comparison::Gte => {
                let window = c.window.as_ref()?;
                match window.reset {
                    Reset::Daily if window.utc_offset_minutes.is_none() => (LimiterKind::Token, LimiterReset::Daily, 0),
                    Reset::Fixed if (legacy::MIN_TOKEN_PERIOD_SECONDS..=legacy::MAX_TOKEN_PERIOD_SECONDS).contains(&window.seconds) =>
                        (LimiterKind::Token, LimiterReset::Fixed, window.seconds),
                    _ => return None,
                }
            }
            _ => return None,
        };
        Some(legacy::LimiterRule {
            kind, reset, period, threshold: c.threshold, enabled: group.enabled,
            action: if group.action == Action::Disable { LimiterAction::Disable } else { LimiterAction::Skip },
        })
    }).collect()
}

pub fn describe_groups(groups: &[Group]) -> String {
    if groups.is_empty() { return "限制器 → 无条件组（不限制）".to_string(); }
    let enabled = groups.iter().filter(|group| group.enabled).count();
    let conditions: usize = groups.iter().map(|group| group.conditions.len()).sum();
    format!("限制器 → {} 个条件组（{enabled} 个启用），{conditions} 条条件", groups.len())
}
