//! 插件申报的定时管理动作；默认值归插件，用户配置与执行记录归宿主。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use agent2api_provider_protocol as proto;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::Notify;

use super::{registry, Host};
use crate::server::logging;

// 可选 RPC 扩展：旧 SDK 返回 NO_METHOD，不改变握手协议与既有插件的兼容性。
const LIST_METHOD: &str = "schedules.list";
pub const MAX_TIMES: usize = 6;

#[derive(Clone, Debug, Deserialize)]
pub struct Definition {
    pub id: String,
    pub enabled: bool,
    /// 北京时间的每日时刻（HH:MM）。
    pub times: Vec<String>,
    pub action: String,
}

#[derive(Default)]
struct Entry {
    definitions: Vec<Definition>,
    unavailable: Option<String>,
    suspended: HashMap<String, String>,
}

fn table() -> &'static Mutex<HashMap<String, Entry>> {
    static TABLE: OnceLock<Mutex<HashMap<String, Entry>>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn notification() -> &'static Notify {
    static CHANGED: OnceLock<Notify> = OnceLock::new();
    CHANGED.get_or_init(Notify::new)
}

pub fn wake() {
    notification().notify_one();
}

pub async fn changed() {
    notification().notified().await;
}

pub fn normalize_time(text: &str) -> Option<String> {
    let (hour, minute) = text.trim().split_once(':')?;
    let hour: u32 = hour.parse().ok()?;
    let minute: u32 = minute.parse().ok()?;
    if hour > 23 || minute > 59 {
        return None;
    }
    Some(format!("{hour:02}:{minute:02}"))
}

fn parse(value: serde_json::Value) -> Result<Vec<Definition>, String> {
    let mut definitions: Vec<Definition> =
        serde_json::from_value(value).map_err(|error| format!("排程申报形状不正确：{error}"))?;
    let mut ids = Vec::new();
    for definition in &mut definitions {
        if definition.id.trim().is_empty()
            || definition.action.trim().is_empty()
            || ids.contains(&definition.id)
            || definition.times.is_empty()
            || definition.times.len() > MAX_TIMES
        {
            return Err("排程申报缺少标识、执行动作或有效时刻，或标识重复".to_string());
        }
        ids.push(definition.id.clone());
        let mut times = Vec::new();
        for time in &definition.times {
            let normalized =
                normalize_time(time).ok_or_else(|| format!("排程时刻「{time}」无效"))?;
            times.push(normalized);
        }
        times.sort();
        times.dedup();
        definition.times = times;
    }
    Ok(definitions)
}

pub(super) async fn load(host: &Host, provider: &str) {
    let result = host
        .call_coded(
            LIST_METHOD,
            json!({ "provider": provider }),
            Duration::from_secs(10),
        )
        .await;
    let entry = match result {
        Ok(value) => match parse(value) {
            Ok(definitions) => Entry {
                definitions,
                ..Entry::default()
            },
            Err(message) => {
                logging::log_with_level("[Plugin]", &message, "warn");
                Entry {
                    unavailable: Some(message),
                    ..Entry::default()
                }
            }
        },
        Err(error) => {
            let message = if matches!(
                error.code.as_deref(),
                Some(proto::WireError::NO_METHOD) | Some(proto::WireError::UNSUPPORTED)
            ) {
                "当前提供商插件未申报定时管理动作，请更新提供商插件".to_string()
            } else {
                let message = format!("提供商插件的排程申报暂不可用：{}", error.message);
                logging::log_with_level("[Plugin]", &message, "warn");
                message
            };
            Entry {
                unavailable: Some(message),
                ..Entry::default()
            }
        }
    };
    if let Ok(mut map) = table().lock() {
        map.insert(provider.to_string(), entry);
    }
}

pub fn definition(provider: &str, id: &str) -> Option<Definition> {
    if !registry::serves(provider) {
        return None;
    }
    table()
        .lock()
        .ok()?
        .get(provider)?
        .definitions
        .iter()
        .find(|definition| definition.id == id)
        .cloned()
}

pub fn pause_reason(provider: &str, id: &str) -> Option<String> {
    if !registry::serves(provider) {
        return Some("未安装或未启用提供商插件".to_string());
    }
    let Some(host) = super::host_for_provider(provider) else {
        return Some("提供商插件尚未加载完成".to_string());
    };
    if host.status().disabled {
        return Some("提供商插件已停用，请在插件页重新启用".to_string());
    }
    let Ok(map) = table().lock() else {
        return Some("提供商插件的排程申报暂不可用".to_string());
    };
    let Some(entry) = map.get(provider) else {
        return Some("提供商插件尚未加载完成".to_string());
    };
    if let Some(message) = &entry.unavailable {
        return Some(message.clone());
    }
    if !entry
        .definitions
        .iter()
        .any(|definition| definition.id == id)
    {
        return Some("当前提供商插件未申报这项排程，请更新提供商插件".to_string());
    }
    entry.suspended.get(id).cloned()
}

/// 能力缺失只在插件重新加载后重试，不能让每天定点的动作退化为 30 秒轮询。
pub fn suspend(provider: &str, id: &str, message: String) {
    if let Ok(mut map) = table().lock() {
        if let Some(entry) = map.get_mut(provider) {
            entry.suspended.insert(id.to_string(), message);
        }
    }
}

pub fn forget(provider: &str) {
    if let Ok(mut map) = table().lock() {
        map.remove(provider);
    }
    wake();
}

pub fn clear_all() {
    if let Ok(mut map) = table().lock() {
        map.clear();
    }
    wake();
}
