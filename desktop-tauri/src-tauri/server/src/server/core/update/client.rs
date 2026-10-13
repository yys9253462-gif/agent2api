//! GitHub 出网：候选出口解析 + 请求头 + 带出口重试的 fetch。
//!
//! 对照 Node 版 workbuddy-update.mjs 的 `resolveEgressCandidates` /
//! `fetchWithEgress` / `githubHeaders` 三块。
//!
//! ─── 候选顺序（设置的出口优先，直连次之，Clash 兜底）──────────
//! 设置页更新面板可以指定出网线路（`updateProxy` 配置键）：
//!   · 指定了出口 → 候选只有它一个 ——「选定出口」的语义就是「走这条线路」，
//!     不再暗中回落直连（回退会让用户以为流量走了所选线路）；
//!   · 未指定（默认）/ 指定但解析失败 → `[{label:'直连', proxy:null}]`，
//!     若 `resolveAccountProxy({source:'clash', listenerUid: CLASH_MIXED_UID})`
//!     成功且没 error、有 port，就再追加一个候选。
//!     解析失败（出口被删 / 被禁 / 数据坏）回退默认候选并留 verbose，
//!     与账号转发「代理不可用回退直连」同一取向：可用性优先，原因留在日志。
//!     Rust 侧走 `core::proxies::resolve_account_proxy` 的同一条路（account_store 里
//!     组会话用的也是它），因此「Clash 未安装/未启用混合端口时只有一个候选」
//!     的行为与 Node 完全一致。
//!
//! ─── 只有网络层失败才换出口 ─────────────────────────────────
//! HTTP 状态码类错误（404 仓库没 Release、403 限额）直接交给调用方判断，
//! 否则 404 会被误当成「需要换代理」而白试一轮。这里 `fetch_with_egress`
//! 只在 `send()` 返回 Err（DNS/TLS/连接/超时）时切下一个候选。

use std::time::Duration;

use serde_json::{json, Value};

use crate::server::config;
use crate::server::core::egress;
use crate::server::core::proxies::{resolve_account_proxy, ProxyResolution, ResolvedProxy, CLASH_MIXED_UID};
use crate::server::logging;

use super::token;
use super::version::UpdateError;
use super::USER_AGENT;

/// GitHub API 要求的头部集合（对应 Node 版 githubHeaders）。
///
/// 令牌来自 `token::effective_token`（「更新设置」里保存的 > 环境变量
/// `GITHUB_TOKEN` / 旧名 `WORKBUDDY_GITHUB_TOKEN`）。配了 token 则限额更高
/// （匿名 60 次/小时，带 token 5000）；两个环境变量名是项目早期（还叫
/// WorkBuddy 网关时）与后来的两套写法，按同一惯例兼容读。
pub fn github_headers() -> Vec<(String, String)> {
    let mut headers = vec![
        ("Accept".to_string(), "application/vnd.github+json".to_string()),
        // UA 与下载走同一个常量（mod.rs 的 USER_AGENT）：项目改名时只改一处
        ("User-Agent".to_string(), USER_AGENT.to_string()),
        ("X-GitHub-Api-Version".to_string(), "2022-11-28".to_string()),
    ];
    if let Some(token) = token::effective_token() {
        headers.push(("Authorization".to_string(), format!("Bearer {token}")));
    }
    headers
}

/// 出网候选：`(标签, 代理配置)`；`proxy` 为 None 表示直连。
///
/// 对应 Node 版 resolveEgressCandidates —— Clash 配置不可读时只有一个候选，
/// 行为与纯直连一致（这里不吞掉整条链路：探测失败只在 verbose 里留痕）。
fn resolve_egress_candidates() -> Vec<(String, Option<ResolvedProxy>)> {
    // 设置里指定了更新出口：只用它（理由见模块头「候选顺序」）
    if let Some(configured) = config::update_proxy() {
        match resolve_account_proxy(Some(&configured)) {
            Some(ProxyResolution::Resolved(proxy)) if proxy.port.is_some() => {
                let label = if proxy.label.is_empty() {
                    "指定的更新出口".to_string()
                } else {
                    proxy.label.clone()
                };
                return vec![(label, Some(proxy))];
            }
            Some(ProxyResolution::Failed(reason)) => {
                logging::verbose(
                    "[Update]",
                    &format!("指定的更新出口不可用（{reason}），回退默认出口"),
                );
            }
            // Resolved 但端口缺失：数据坏了（手工改库写出非法端口），
            // 与解析失败同路 —— 回退默认候选，不去连一个连不上的出口
            _ => {}
        }
    }
    let mut candidates = vec![("直连".to_string(), None)];
    let resolution = resolve_account_proxy(Some(&json!({
        "source": "clash",
        "listenerUid": CLASH_MIXED_UID,
    })));
    match resolution {
        Some(resolution) => {
            if let Some(proxy) = resolution.resolved() {
                if proxy.port.is_some() {
                    let label = if proxy.label.is_empty() {
                        "Clash 混合端口".to_string()
                    } else {
                        proxy.label.clone()
                    };
                    candidates.push((label, Some(proxy.clone())));
                }
            } else if let Some(error) = resolution.error() {
                // Node 在这条分支上静默（try/catch 吞掉），这里留 verbose 便于排障
                logging::verbose("[Update]", &format!("Clash 出口不可用（{error}），仅用直连"));
            }
        }
        None => {}
    }
    candidates
}

/// 依次尝试各出口，返回第一个**连得上**的响应（HTTP 状态码交给调用方判断）。
///
/// 全部出口都在网络层失败时返回 502「无法连接 GitHub：<错误链>」
/// （文案照抄 Node 的 `无法连接 GitHub：${proxyErrorDetail(lastError)}`）。
pub async fn fetch_with_egress(
    url: &str,
    headers: &[(String, String)],
    timeout_ms: Option<u64>,
) -> Result<reqwest::Response, UpdateError> {
    fetch_via_candidates("[Update]", url, headers, timeout_ms)
        .await
        .map_err(|detail| UpdateError::new(format!("无法连接 GitHub：{detail}")))
}

/// 非 GitHub 的**公网**端点出网（目前只有 models.dev 模型元数据用它）。
///
/// 与 [`fetch_with_egress`] 是同一份候选出口实现、同一套「只有网络层失败才换
/// 出口」的判定，唯一差别在失败文案：那条链路的所有调用方都是 GitHub，文案写死
/// 了「无法连接 GitHub」；这里从 URL 取主机名 —— 对着 models.dev 报「无法连接
/// GitHub」会把排查方向直接带偏。日志标签也换成调用方的（`[Models]`）。
///
/// 沿用 `updateProxy` / Clash / 直连这条既有链路而不是新加一个配置键：它回答的
/// 问题就是「这台机器的公网怎么出去」，与具体访问哪个公网站点无关。
pub async fn fetch_public(
    url: &str,
    headers: &[(String, String)],
    timeout_ms: Option<u64>,
) -> Result<reqwest::Response, String> {
    let host = url::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_string))
        .unwrap_or_else(|| url.to_string());
    fetch_via_candidates("[Models]", url, headers, timeout_ms)
        .await
        .map_err(|detail| format!("无法连接 {host}：{detail}"))
}

/// 候选出口依次尝试的共用实现：返回第一个连上的响应，全部失败时把最后一条
/// 错误链作为 `Err(String)` 交给调用方去拼自己的文案（见上面两个函数）。
async fn fetch_via_candidates(
    tag: &str,
    url: &str,
    headers: &[(String, String)],
    timeout_ms: Option<u64>,
) -> Result<reqwest::Response, String> {
    let candidates = resolve_egress_candidates();
    let multiple = candidates.len() > 1;
    let mut last_error: Option<reqwest::Error> = None;
    for (label, proxy) in &candidates {
        let client = egress::client_for(proxy.as_ref());
        let mut builder = client.get(url);
        // 请求级总超时只给 API 探测用（响应是小 JSON）。reqwest 0.12 的
        // `.timeout()` 覆盖到 body 读完为止 —— 下载传 None，安装包几十上百 MB
        // 不可能 30 秒内下完；卡死场景由 egress 客户端的 read_timeout 兜底
        if let Some(timeout_ms) = timeout_ms {
            builder = builder.timeout(Duration::from_millis(timeout_ms));
        }
        for (name, value) in headers {
            builder = builder.header(name.as_str(), value.as_str());
        }
        match builder.send().await {
            Ok(response) => {
                if multiple && proxy.is_some() {
                    logging::verbose(tag, &format!("经 {label} 访问成功"));
                }
                return Ok(response);
            }
            Err(error) => {
                logging::verbose(
                    tag,
                    &format!("经 {label} 访问失败: {}", egress::describe_error_detail(&error)),
                );
                last_error = Some(error);
            }
        }
    }
    Err(match &last_error {
        Some(error) => egress::describe_error_detail(error),
        None => "未知错误".to_string(),
    })
}

/// 出网候选的展示形态（供日志/排障；不含凭证）。
/// 对应 Node 版 workbuddy-update.mjs 内部的 `resolveEgressCandidates()` 结果形态
/// （Node 侧为内部细节，这里作为排障口暴露，保留）。
#[allow(dead_code)]
pub fn describe_candidates() -> Value {
    let list: Vec<Value> = resolve_egress_candidates()
        .into_iter()
        .map(|(label, proxy)| {
            json!({
                "label": label,
                "proxy": egress::describe_public(proxy.as_ref()),
            })
        })
        .collect();
    Value::Array(list)
}
