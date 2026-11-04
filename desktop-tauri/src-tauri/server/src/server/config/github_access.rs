use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{update_raw_field, with_raw};

const KEY_GITHUB_ACCESS: &str = "githubAccess";
pub const DEFAULT_GITHUB_MIRROR: &str = "https://gh-proxy.com/";

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GithubAccessMode {
    #[default]
    Auto,
    Official,
    Accelerated,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct GithubAccessSettings {
    pub mode: GithubAccessMode,
    pub mirror_url: String,
}

impl Default for GithubAccessSettings {
    fn default() -> Self {
        Self {
            mode: GithubAccessMode::Auto,
            mirror_url: DEFAULT_GITHUB_MIRROR.to_string(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GithubAccessPatch {
    pub mode: Option<GithubAccessMode>,
    pub mirror_url: Option<String>,
}

fn normalize_mirror(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(DEFAULT_GITHUB_MIRROR.to_string());
    }
    if value.len() > 1024 || value.contains('\\') || value.chars().any(char::is_whitespace) {
        return Err("GitHub 加速前缀不是合法地址".to_string());
    }
    let mut url = url::Url::parse(value).map_err(|_| "GitHub 加速前缀不是合法地址".to_string())?;
    if url.scheme() != "https" || url.host_str().is_none() {
        return Err("GitHub 加速前缀必须是 HTTPS 地址".to_string());
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path().contains("://")
        || url.path().to_ascii_lowercase().contains("%3a")
    {
        return Err("GitHub 加速前缀不能包含凭证、查询参数或原始 GitHub 地址".to_string());
    }
    if matches!(
        url.host_str(),
        Some("github.com" | "api.github.com" | "raw.githubusercontent.com")
    ) {
        return Err("GitHub 加速前缀应填写加速站地址，而不是 GitHub 官方地址".to_string());
    }
    let path = format!("{}/", url.path().trim_end_matches('/'));
    url.set_path(&path);
    Ok(url.to_string())
}

pub fn github_access() -> GithubAccessSettings {
    let raw = with_raw(|raw| raw.get(KEY_GITHUB_ACCESS).cloned()).flatten();
    let mut settings = raw
        .and_then(|value| serde_json::from_value::<GithubAccessSettings>(value).ok())
        .unwrap_or_default();
    settings.mirror_url = normalize_mirror(&settings.mirror_url)
        .unwrap_or_else(|_| DEFAULT_GITHUB_MIRROR.to_string());
    settings
}

pub fn set_github_access(
    patch: GithubAccessPatch,
) -> Result<(GithubAccessSettings, bool, bool), String> {
    if patch.mode.is_none() && patch.mirror_url.is_none() {
        return Err("请提供 GitHub 访问方式或加速前缀".to_string());
    }
    let previous = github_access();
    let mut next = previous.clone();
    if let Some(mode) = patch.mode {
        next.mode = mode;
    }
    if let Some(mirror) = patch.mirror_url {
        next.mirror_url = normalize_mirror(&mirror)?;
    }
    let changed = next != previous;
    let value: Value = serde_json::to_value(&next).map_err(|error| error.to_string())?;
    let saved = update_raw_field(KEY_GITHUB_ACCESS, value);
    Ok((next, saved, changed))
}
