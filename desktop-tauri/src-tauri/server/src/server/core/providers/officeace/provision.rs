//! OfficeAce OAuth 的后两段：`project_id` 补查与「开通」。
//!
//! ── 为什么单独一份（依据：本机 OfficeAce 的 `green-package/api/dist/auth/
//! huawei-oauth.js`，即上游官方实现）──────────────────────────
//! 官方在 `client-permission-validate` 之后还有两步，本仓此前缺了它们 ——
//! 结果就是「账号已开通也报没套餐、未开通则直接失败且没有开通的路」：
//!
//!   · **① `project_id` 补查**：IAM OAuth2 换来的临时凭据实测只有
//!     `access_key_id` / `secret_access_key` / `security_token` / `expiration`
//!     （本机 OfficeAce 的 IAMToken 响应日志逐字如此），`project_id` 要另打一发
//!     IAM keystone 的 `GET {iam}/v3/projects` 按 region 名匹配（官方
//!     `resolveProjectId`，匹配不到还会用 VPC 探针触发项目自动创建）。控制面
//!     后续请求都签 `X-Project-ID`，缺了它这一发能过、开通那一发就少签一个头。
//!   · **② 开通**：validate 响应里 `model_info` 为空**不是错误**，而是「账号
//!     尚未开通」的正常形态。官方此时打 `POST {claw}/v1/claw/client-subscription`
//!     （body `{}`，或带用户输入的 `promotion_code`），成功即返回 `model_info`。
//!     官方只有拿到邀请码类错误码时才让用户输入邀请码。
//!
//! ── 实测（2026-10-13，本机已开通账号、真实临时凭据）───────────
//!   · validate：`model_info` 嵌在 `subscription.model_info` 下；不带
//!     `x-subscription-type: v2` 也能拿到它（两种都试过，200）——官方带，照带；
//!   · `client-subscription`：`model_info` 在**顶层**，body `{}` 对已开通账号
//!     幂等返回完整 model_info；
//!   · `x-subscription-type: v2` 是控制面订阅族的通行头（与 `subscription.rs`
//!     的模块头同源）。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use std::time::Duration;

use serde_json::Value;

use crate::server::core::egress;
use crate::server::core::providers::officeace::signer::{
    self, Algorithm, Credential as SigningCredential,
};

/// IAM keystone 基址（官方 `DEFAULT_IAM_KEYSTONE_BASE`）。
const IAM_KEYSTONE_BASE: &str = "https://iam.myhuaweicloud.com";
/// 单次请求超时（与 `oauth.rs` 的 `REQUEST_TIMEOUT_MS` 同值）。
const REQUEST_TIMEOUT_MS: u64 = 30_000;
/// 控制面订阅族的通行头（官方 `checkPermission` / `subscriptionClaw` 都带）。
pub const SUBSCRIPTION_TYPE_HEADER: (&str, &str) = ("x-subscription-type", "v2");

/// 邀请码族错误码 → 给用户的中文提示（官方 `PROMOTION_CODE_ERROR_MESSAGES` 的中文口径）。
///
/// 官方把这些码当「要让用户输入邀请码」的信号；本仓还没有邀请码输入这一路，
/// 所以提示落到「先去官方客户端完成开通」——比「没有可用套餐」准确得多。
fn promotion_hint(error_code: &str) -> Option<&'static str> {
    match error_code {
        "AgentArts.11000008" => Some("邀请码无效"),
        "AgentArts.11000009" => Some("邀请码不存在或今日配额已用完"),
        "AgentArts.00000104" | "common.01010003" => Some("当前账号权限不足"),
        "common.01010004" => Some("账号可能未实名认证或处于欠费状态"),
        _ => None,
    }
}

/// ① 补查 `project_id`：`GET {iam}/v3/projects` 里按 region 名匹配启用中的项目。
///
/// 返回 `None` = 没查到（凭据不可用 / 网络失败 / 没有该 region 的项目）——
/// 调用方如实把空串留给下游，并且**不**把它当登录失败：validate 那一发不缺它也过。
pub async fn resolve_project_id(credential: &SigningCredential, region: &str) -> Option<String> {
    let url = format!("{IAM_KEYSTONE_BASE}/v3/projects");
    let headers = signer::sign(&signer::SignRequest {
        algorithm: Algorithm::Sdk,
        method: "GET",
        url: &url,
        headers: &[],
        body: b"",
        credential,
        region,
        date_override: None,
    })
    .ok()?;
    let mut request = egress::client_for(None)
        .get(&url)
        .timeout(Duration::from_millis(REQUEST_TIMEOUT_MS));
    for (name, value) in &headers {
        request = request.header(name.as_str(), value.as_str());
    }
    let response = request.send().await.ok()?;
    if !(200..300).contains(&response.status().as_u16()) {
        return None;
    }
    let payload = response.json::<Value>().await.ok()?;
    let projects = payload.get("projects").and_then(Value::as_array)?;
    projects.iter().find_map(|project| {
        let name = project.get("name").and_then(Value::as_str)?;
        let id = project.get("id").and_then(Value::as_str)?;
        let enabled = project
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        (name == region && enabled && !id.is_empty()).then(|| id.to_string())
    })
}

/// ② 开通：`POST {claw}/v1/claw/client-subscription`，成功返回**顶层** `model_info`。
///
/// `promotion_code` 为 `None` 时 body 发 `{}`（官方首次开通就是这个形状）；
/// 失败时返回面向用户的中文错误（邀请码族错误码单独给准确提示）。
pub async fn subscribe_model_info(
    cloud_base: &str,
    credential: &SigningCredential,
    promotion_code: Option<&str>,
) -> Result<Value, String> {
    let url = format!("{cloud_base}/v1/claw/client-subscription");
    let body = match promotion_code.map(str::trim).filter(|code| !code.is_empty()) {
        Some(code) => serde_json::json!({ "promotion_code": code }).to_string(),
        None => "{}".to_string(),
    };
    let headers = signer::sign(&signer::SignRequest {
        algorithm: Algorithm::Sdk,
        method: "POST",
        url: &url,
        headers: &[
            (
                "Content-Type".to_string(),
                "application/json;charset=utf8".to_string(),
            ),
            (
                SUBSCRIPTION_TYPE_HEADER.0.to_string(),
                SUBSCRIPTION_TYPE_HEADER.1.to_string(),
            ),
        ],
        body: body.as_bytes(),
        credential,
        region: super::oauth::REGION,
        date_override: None,
    })?;
    let mut request = egress::client_for(None)
        .post(&url)
        .timeout(Duration::from_millis(REQUEST_TIMEOUT_MS))
        .body(body);
    for (name, value) in &headers {
        request = request.header(name.as_str(), value.as_str());
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("开通请求失败：{}", egress::describe_error_detail(&error)))?;
    let status = response.status().as_u16();
    let payload = response
        .json::<Value>()
        .await
        .unwrap_or(Value::Null);
    if !(200..300).contains(&status) {
        let error_code = payload
            .get("error_code")
            .and_then(Value::as_str)
            .unwrap_or("");
        let detail = payload
            .get("error_msg")
            .and_then(Value::as_str)
            .or_else(|| payload.get("error_message").and_then(Value::as_str))
            .unwrap_or("");
        if let Some(hint) = promotion_hint(error_code) {
            return Err(format!(
                "该账号尚未开通果办服务（{hint}）：请先在 OfficeAce 客户端完成开通，再回来重新登录"
            ));
        }
        let shown = if detail.is_empty() { error_code } else { detail };
        return Err(format!("开通失败（HTTP {status}）：{shown}"));
    }
    payload
        .get("model_info")
        .filter(|value| value.is_object())
        .cloned()
        .ok_or_else(|| "开通成功但云端仍未返回模型网关信息，请稍后重试".to_string())
}
