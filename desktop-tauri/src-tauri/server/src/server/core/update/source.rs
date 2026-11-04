use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde_json::json;
use sha2::{Digest, Sha256};

use crate::server::config::{self, GithubAccessMode};
use crate::server::core::task_state;
use crate::server::logging;

use super::token;

const PREFERENCE_MS: i64 = 10 * 60_000;

#[derive(Clone, Copy)]
pub enum GithubSettings {
    Update,
    Plugin,
}

impl GithubSettings {
    pub fn rate_limit_message(self, retry_at: i64) -> String {
        self.message_for_token(retry_at, token::effective_token().is_some())
    }

    fn message_for_token(self, retry_at: i64, filled: bool) -> String {
        if !filled {
            return match self {
                Self::Update => "GitHub 接口限流中，请在「更新设置」的「GitHub 令牌」中填写凭证，以提高请求限额。",
                Self::Plugin => "GitHub 接口限流中，请在「插件设置」的「GitHub 令牌」中填写凭证，以提高请求限额。",
            }
            .to_string();
        }
        let seconds = retry_at.saturating_sub(logging::now_ms()).max(0).saturating_add(999) / 1000;
        if seconds > 0 {
            format!("GitHub 接口限流中，请在 {seconds} 秒后重试")
        } else {
            "GitHub 接口限流中，请稍后重试".to_string()
        }
    }

    pub fn describe_error(self, message: &str, retry_at: i64) -> String {
        if !is_rate_limit_error(message) {
            return message.to_string();
        }
        let filled = token::effective_token().is_some();
        if filled && retry_at == 0 {
            if let Some(seconds) = message
                .split_once("请在 ")
                .and_then(|(_, tail)| tail.split_once(" 秒后重试"))
                .and_then(|(seconds, _)| seconds.parse::<i64>().ok())
                .filter(|seconds| *seconds > 0)
            {
                return format!("GitHub 接口限流中，请在 {seconds} 秒后重试");
            }
        }
        self.message_for_token(retry_at, filled)
    }

    pub fn waiting_message(self, state: &task_state::TaskState) -> String {
        if !state.running() && state.retry_at > logging::now_ms() {
            if let Some(error) = state.last_error.as_deref().filter(|error| is_rate_limit_error(error)) {
                return self.describe_error(error, state.retry_at);
            }
        }
        state.waiting_message()
    }
}

fn is_rate_limit_error(message: &str) -> bool {
    [
        "GitHub 接口限流中",
        "GitHub 接口处于限流冷却期",
        "GitHub 接口访问受限",
        "查询插件更新失败：GitHub 返回 HTTP 403",
        "查询插件更新失败：GitHub 返回 HTTP 429",
    ]
    .iter()
    .any(|prefix| message.contains(prefix))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Official,
    Mirror,
}

#[derive(Clone, Debug)]
pub struct Candidate {
    pub url: String,
    pub original: String,
    pub source: Source,
    key: String,
    mirror_origin: Option<url::Url>,
}

struct Preference {
    source: Source,
    until: i64,
}

fn preferences() -> &'static Mutex<HashMap<String, Preference>> {
    static MEMORY: OnceLock<Mutex<HashMap<String, Preference>>> = OnceLock::new();
    MEMORY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn clear_preferences() {
    if let Ok(mut memory) = preferences().lock() {
        memory.clear();
    }
}

pub fn is_official_api(value: &str) -> bool {
    url::Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str() == Some("api.github.com")
            && url.port_or_known_default() == Some(443)
            && url.username().is_empty()
            && url.password().is_none()
    })
}

pub fn is_resource(value: &str) -> bool {
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    if url.scheme() != "https"
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return false;
    }
    match url.host_str().unwrap_or("") {
        "api.github.com" => url.path().starts_with("/repos/"),
        "raw.githubusercontent.com" | "codeload.github.com" => true,
        "github.com" | "www.github.com" => url.path().contains("/releases/download/"),
        host => host.ends_with(".githubusercontent.com"),
    }
}

fn preference_key(value: &str) -> String {
    let settings = config::github_access();
    let host = url::Url::parse(value)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_default();
    let proxy = config::update_proxy();
    let identity = serde_json::to_vec(&(host, settings, proxy)).unwrap_or_default();
    format!("{:x}", Sha256::digest(identity))
}

pub fn candidates(value: &str, allow_mirror: bool) -> Vec<Candidate> {
    let settings = config::github_access();
    let key = preference_key(value);
    let official = Candidate {
        url: value.to_string(),
        original: value.to_string(),
        source: Source::Official,
        key: key.clone(),
        mirror_origin: None,
    };
    if !allow_mirror || settings.mode == GithubAccessMode::Official || !is_resource(value) {
        return vec![official];
    }
    let mirror = Candidate {
        url: format!("{}{value}", settings.mirror_url),
        original: value.to_string(),
        source: Source::Mirror,
        key: key.clone(),
        mirror_origin: url::Url::parse(&settings.mirror_url).ok(),
    };
    let remembered = preferences().lock().ok().and_then(|memory| {
        memory
            .get(&key)
            .filter(|item| item.until > logging::now_ms())
            .map(|item| item.source)
    });
    let mirror_first = settings.mode == GithubAccessMode::Accelerated
        || (settings.mode == GithubAccessMode::Auto && remembered == Some(Source::Mirror));
    if mirror_first {
        vec![mirror, official]
    } else {
        vec![official, mirror]
    }
}

impl Candidate {
    pub fn label(&self) -> &'static str {
        match self.source {
            Source::Official => "官方源",
            Source::Mirror => "加速源",
        }
    }

    pub fn allows_redirect(&self, value: &url::Url) -> bool {
        if !value.username().is_empty() || value.password().is_some() {
            return false;
        }
        if self.source == Source::Mirror {
            if value.scheme() != "https" {
                return false;
            }
            let official = if is_official_api(&self.original) {
                is_official_api(value.as_str())
            } else {
                is_resource(value.as_str())
            };
            return official
                || self.mirror_origin.as_ref().is_some_and(|base| {
                    value.host_str() == base.host_str()
                        && value.port_or_known_default() == base.port_or_known_default()
                });
        }
        if is_official_api(&self.original) {
            return is_official_api(value.as_str());
        }
        if is_resource(&self.original) {
            return is_resource(value.as_str());
        }
        url::Url::parse(&self.original).is_ok_and(|original| {
            value.scheme() == original.scheme()
                && value.host_str() == original.host_str()
                && value.port_or_known_default() == original.port_or_known_default()
        })
    }

    pub fn remember(&self) {
        if self.source == Source::Official {
            return;
        }
        let now = logging::now_ms();
        if let Ok(mut memory) = preferences().lock() {
            memory.retain(|_, item| item.until > now);
            if memory
                .get(&self.key)
                .is_some_and(|item| item.source == self.source)
            {
                return;
            }
            memory.insert(
                self.key.clone(),
                Preference {
                    source: self.source,
                    until: now + PREFERENCE_MS,
                },
            );
        }
    }

    pub fn forget(&self) {
        if let Ok(mut memory) = preferences().lock() {
            if memory
                .get(&self.key)
                .is_some_and(|item| item.source == self.source)
            {
                memory.remove(&self.key);
            }
        }
    }

    fn rate_key(&self, headers: &[(String, String)]) -> Option<String> {
        if !is_official_api(&self.original) {
            return None;
        }
        let auth = if self.source == Source::Official {
            headers
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("authorization"))
                .map(|(_, value)| value.as_str())
                .unwrap_or("")
        } else {
            ""
        };
        let proxy = config::update_proxy();
        let origin = if self.source == Source::Mirror {
            self.mirror_origin
                .as_ref()
                .map(|url| url.as_str())
                .unwrap_or("")
        } else {
            "https://api.github.com"
        };
        let bytes = serde_json::to_vec(&(origin, proxy, auth)).unwrap_or_default();
        Some(format!("githubRate:{:x}", Sha256::digest(bytes)))
    }

    pub fn cooling_message(&self, headers: &[(String, String)]) -> Option<String> {
        let state = task_state::read(&self.rate_key(headers)?).ok()?;
        let until = state.value.as_ref()?.get("retryAt")?.as_i64()?;
        (until > logging::now_ms()).then(|| GithubSettings::Update.rate_limit_message(until))
    }

    pub fn record_rate_limit(&self, headers: &[(String, String)], response: &reqwest::Response) {
        let Some(key) = self.rate_key(headers) else {
            return;
        };
        let until = retry_deadline(response.headers(), response.status().as_u16());
        if until > logging::now_ms() {
            if let Err(error) = task_state::store_value(&key, json!({ "retryAt": until })) {
                logging::verbose("[Update]", &error);
            }
        }
    }
}

pub fn retry_deadline(headers: &reqwest::header::HeaderMap, status: u16) -> i64 {
    let now = logging::now_ms();
    let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
    let after = header("retry-after")
        .and_then(|value| {
            value
                .parse::<i64>()
                .ok()
                .map(|seconds| now.saturating_add(seconds.max(0).saturating_mul(1000)))
                .or_else(|| {
                    chrono::DateTime::parse_from_rfc2822(value)
                        .ok()
                        .map(|date| date.timestamp_millis())
                })
        })
        .unwrap_or(0);
    let reset = if header("x-ratelimit-remaining") == Some("0") {
        header("x-ratelimit-reset")
            .and_then(|value| value.parse::<i64>().ok())
            .map(|seconds| seconds.saturating_mul(1000).saturating_add(1000))
            .unwrap_or(0)
    } else {
        0
    };
    after.max(reset).max(if status == 403 || status == 429 {
        now + 60_000
    } else {
        0
    })
}

pub fn should_switch(candidate: &Candidate, status: reqwest::StatusCode) -> bool {
    status.is_server_error() || (candidate.source == Source::Mirror && status.as_u16() == 404)
}
