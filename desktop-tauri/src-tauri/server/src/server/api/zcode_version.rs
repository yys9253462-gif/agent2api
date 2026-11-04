//! ZCode 客户端版本号：`GET/PUT /api/zcode/version`。
//!
//! ── 这个端点为什么存在（这是用户能看见的一格）──────────────
//! ZCode 的活动套餐（限时体验礼包，如 2026-10 的「ZCode 1024 Dev Day」）
//! 由上游按**客户端版本号**下发：`GET billing/preview?app_version=…` 只在
//! 版本号达到当期门槛时才返回 `data.plans`，否则回一份**空列表**
//! （`code: 0`，不是错误 —— 这是「看起来没有活动」的成因）。
//!
//! 2026-10-23 实测的门槛（匿名探测，三个平台一致）：
//!
//! | app_version | `data.plans` |
//! |---|---|
//! | 3.13.0 / 3.14.0 / 3.14.3 / 3.15.0 / 3.15.1 / 3.16.0 | `[]` |
//! | 3.16.1 / 3.16.2 / 3.17.0 | 含 `zcode-v3-start-plan-1024` |
//!
//! 门槛随每期活动上移，而**落后时的症状是静默空列表**。插件里那份默认值
//! 是编译期常量（`zcode::claim::DEFAULT_APP_VERSION`），每期都要跟着改、
//! 跟着发版。这个端点是那条链的旁路：用户遇到「活动在但探不到」时，
//! 在这里填一个更新的版本号即可，不必等发版。
//!
//! ── 配置存在哪 ──────────────────────────────────────────────
//! `plugin.zcode.appVersion`（与插件 `settings_get("zcode", "appVersion")`
//! 读的是同一个键，见 `plugin_host::plugin_settings_key`）。
//!
//! ── 为什么不用 `settings.set` 走插件 ────────────────────────
//! 插件也会写设置（`settings_set`），但这一格是**网关侧的用户配置**：
//! 界面写入、插件只读。让界面走插件的写接口会把「配置归谁管」弄反 ——
//! 而且插件没装时这一格仍然要能读能写（用户的配置不该跟插件包一起消失）。
//!
//! ── 优先级 ──────────────────────────────────────────────────
//! 环境变量 `ZCODE_APP_VERSION` > 本设置 > 插件内置默认值。
//! 环境变量排最前是排障口径：它由**部署者**设定，不该被界面悄悄盖掉。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use axum::body::Bytes;
use axum::extract::State;
use axum::response::Response;
use serde_json::{json, Value};

use crate::server::config;
use crate::server::errors;
use crate::server::http::{ok_json, parse_body};
use crate::server::ServerState;

/// 配置键（落 `config.json` 的 `plugin.zcode.appVersion`）。
///
/// 与插件侧 `claim::SETTING_APP_VERSION` + `APP_VERSION_SETTING_OWNER`
/// 拼出来的路径**必须逐字一致** —— 两处写岔的表现是「界面上填了、
/// 插件读不到」，而插件那边只会安静地回落默认值。
const KEY: &str = "plugin.zcode.appVersion";

/// 版本号长度上限（`3.16.2` 这类语义化版本不会超过它）。
const MAX_LEN: usize = 32;

/// GET /api/zcode/version —— 当前值与环境变量覆盖情况。
///
/// 响应形状（前端据此渲染输入框与提示）：
/// ```json
/// { "version": "3.16.2", "envOverride": null }
/// ```
/// `version` 是**当前设置的值**（未设置时为空串，表示「用插件内置默认值」）；
/// `envOverride` 非空时说明环境变量正压着它 —— 界面据此提示「界面上的值
/// 此刻不生效」，而不是让用户对着一个改了没反应的输入框发懵。
pub async fn get_version(State(_state): State<ServerState>) -> Response {
    ok_json(json!({
        "version": stored_version().unwrap_or_default(),
        "envOverride": env_override(),
    }))
}

/// PUT /api/zcode/version —— body `{"version": "3.16.2"}`（空串 = 清空覆盖）。
///
/// 校验只做两件事：长度与字符集。**不校验「这个版本存在」** —— 上游的版本
/// 号对我们是黑盒（门槛在服务端，客户端拿不到清单），在校验里编一套规则
/// 只会在上游改号规则时误伤。长度与字符集必须拦：它会被拼进 URL 的查询串
/// （`?app_version=…`），带 `&` 或空格会拼出畸形 URL。
pub async fn put_version(State(_state): State<ServerState>, body: Bytes) -> Response {
    let payload = match parse_body(&body) {
        Ok(value) => value,
        Err(error) => return errors::management_error(400, error.message),
    };
    let Some(object) = payload.as_object() else {
        return errors::management_error(400, "请求体必须是 JSON 对象");
    };
    let Some(raw) = object.get("version") else {
        return errors::management_error(400, "缺少 version");
    };
    let Some(text) = raw.as_str() else {
        return errors::management_error(400, "version 必须是字符串");
    };
    let trimmed = text.trim();
    if let Err(message) = validated(trimmed) {
        return errors::management_error(400, message);
    }
    let next = if trimmed.is_empty() {
        Value::Null
    } else {
        Value::String(trimmed.to_string())
    };
    if !config::update_raw_field(KEY, next) {
        crate::server::logging::log("[Config]", "⚠️  ZCode 客户端版本号写入失败，本次运行内仍生效");
    }
    crate::server::logging::log(
        "[Config]",
        if trimmed.is_empty() {
            "ZCode 客户端版本号已清空（回落插件内置默认值）".to_string()
        } else {
            format!("ZCode 客户端版本号设为 {trimmed}（下一次探测/领取生效）")
        }
        .as_str(),
    );
    ok_json(json!({
        "version": trimmed,
        "envOverride": env_override(),
    }))
}

/// 校验版本号（空串是合法输入：表示清空覆盖）。
///
/// 字符集取 `[0-9A-Za-z._-]`：覆盖 `3.16.2`、`3.16.2-beta.1` 这类形态，
/// 同时排除会破坏查询串的字符（`&`、`#`、空格、中文）。
fn validated(text: &str) -> Result<(), String> {
    if text.len() > MAX_LEN {
        return Err(format!("版本号最多 {MAX_LEN} 个字符"));
    }
    if text.is_empty() {
        return Ok(());
    }
    let legal = text
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'));
    if !legal {
        return Err("版本号只能包含字母、数字与 . _ -（它会拼进请求 URL）".to_string());
    }
    // 首字符必须是数字：上游的形态恒为 `3.x.y`，允许 `v3.16.2` 之类的写法
    // 只会在拼进查询串时被上游判成无效（且那种失败是空列表，很难查）。
    if !text.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        return Err("版本号必须以数字开头（形如 3.16.2）".to_string());
    }
    Ok(())
}

/// 读配置里存的值（缺失 / 非字符串 / 空串 → `None`）。
fn stored_version() -> Option<String> {
    config::with_raw(|raw| {
        raw.get(KEY)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string)
    })
    .flatten()
}
/// 环境变量 `ZCODE_APP_VERSION` 的值（非空才返回）。
///
/// 它比本设置优先级高（见模块头）：这里回显它，是为了让界面能说清
/// 「你填的值此刻被环境变量压着」。
fn env_override() -> Option<String> {
    std::env::var("ZCODE_APP_VERSION")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_legal_and_clears_the_override() {
        assert!(validated("").is_ok());
    }

    #[test]
    fn a_plain_semver_is_accepted() {
        for text in ["3.16.2", "3.16.2-beta.1", "3.17.0", "4.0"] {
            assert!(validated(text).is_ok(), "{text} 应当通过");
        }
    }

    #[test]
    fn characters_that_break_a_query_string_are_rejected() {
        // `&` 会截断查询串、空格要编码、中文上游不认 —— 都在这一层拦掉
        for text in ["3.16.2&platform=x", "3.16 2", "3.16.2版", "3.16.2\n"] {
            assert!(validated(text).is_err(), "{text:?} 不该通过");
        }
    }

    #[test]
    fn a_version_must_start_with_a_digit() {
        // `v3.16.2` 会被上游判成无效，症状是空列表而不是报错
        assert!(validated("v3.16.2").is_err());
        assert!(validated("3.16.2").is_ok());
    }

    #[test]
    fn an_over_long_version_is_rejected() {
        let long = "3.".to_string() + &"1".repeat(MAX_LEN);
        assert!(validated(&long).is_err());
    }
}
