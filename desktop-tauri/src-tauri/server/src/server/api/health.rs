//! GET /health —— 壳侧就绪探测与前端首屏健康摘要。
//!
//! shape 逐字段对齐 Node 版 server.mjs handleHealth（562-584 行）：
//!   status 按「上游是否配置」（configured 才 ok，否则 degraded）/
//!   upstreamBaseUrl / authSource / currentAccountId /
//!   tokenExpiresAt / canRefresh 全来自 `auth.get_config_summary()`；
//!   login 为 `auth.get_status()` 的完整形态（已登录/未登录两分支）；
//!   authRequired 表示模型调用始终需要网关 Key / defaultModel / models 数量。
//!
//! `authApiBase` / `chatApiUrl` 两栏已删除（2026-10 拆家）：它们是**上游端点**，
//! 随「怎么打上游」的知识一起搬到了提供商插件，网关不再持有。字段整体移除
//! 而不是给 null —— 消费方（壳侧探针、前端）都不读它们。
//!
//! 注意 /health 返回**裸对象**（不是 `{success,data}` 信封）—— Node 版这里
//! 用的是 sendJson 直接发对象，与 /api/* 不同。
//!
//! `get_config_summary()` 是纯本地的（只查凭证是否存在，不发网络请求），
//! 所以 health 不需要等转发链路。
//!
//! `models` 是目录条数（切片 4 起为真值）：Node 版取 `modelCatalog.list().length`，
//! 目录未刷新过时就是内置清单的条数，刷新后是远程清单的条数 ——
//! 这个字段只反映「当前内存里的目录」，不触发刷新（刷新由启动流程与
//! GET /v1/models 负责，/health 保持纯本地语义）。

use axum::extract::State;
use axum::response::Response;
use serde_json::{json, Value};

use crate::server::config;
use crate::server::http::raw_json;
use crate::server::ServerState;

/// 上游未配置时的说明文案。
///
/// 与 `core::auth::UNCONFIGURED_REASON` 同一份 —— Node 版这条有两处措辞：
/// `getConfigSummary()` 的「尚未登录（node server.mjs --login）」与启动横幅的
/// 「暂无可用登录态」。壳内 Rust 版没有那个命令行入口（登录走 UI 的
/// /api/session/login/*），指向一个不存在的命令只会误导用户，因此统一取后者。
pub const UNCONFIGURED_REASON: &str = crate::server::core::auth::UNCONFIGURED_REASON;

/// 处理 GET /health
pub async fn handle(State(state): State<ServerState>) -> Response {
    // headless 的安全形态（未注册闸门 / /v1 fail-closed）下只回答「进程活着」：
    // 完整摘要会把上游地址、登录状态、模型数量递给公网上的探测者。
    // 探活方（Docker HEALTHCHECK / 反代）只看状态码与 status 字段，不受影响；
    // 桌面壳与面板首屏不受影响 —— 那两种形态下闸门与 fail-closed 都不会开启。
    if crate::server::access::panel_gate() || crate::server::access::v1_fail_closed() {
        return raw_json(json!({ "status": "ok" }));
    }
    let snapshot = config::current();
    let summary = state.auth().get_config_summary();
    let status = state.auth().get_status();
    let healthy = summary
        .get("configured")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    raw_json(json!({
        "status": if healthy { "ok" } else { "degraded" },
        "transport": "upstream-api",
        "product": "WorkBuddy",
        "upstreamConfigured": healthy,
        "upstreamBaseUrl": summary.get("baseUrl").cloned().unwrap_or(Value::Null),
        "authSource": summary.get("authSource").cloned().unwrap_or(Value::Null),
        "currentAccountId": summary.get("currentAccountId").cloned().unwrap_or(Value::Null),
        "tokenExpiresAt": summary.get("tokenExpiresAt").cloned().unwrap_or(Value::Null),
        "canRefresh": summary.get("canRefresh").and_then(Value::as_bool).unwrap_or(false),
        "login": status,
        "unavailableReason": if healthy {
            Value::Null
        } else {
            summary
                .get("unavailableReason")
                .cloned()
                .unwrap_or_else(|| Value::String(UNCONFIGURED_REASON.to_string()))
        },
        "authRequired": true,
        "defaultModel": snapshot.default_model(),
        // 模型目录条数（对照 Node 的 `models: modelCatalog.list().length`）。
        // 本字段是「唯一上游时代」的契约读数：那时进程里只有 workbuddy 一份
        // 目录。现在目录由各家适配器在读取时聚合（见 `core::providers::catalog`），
        // 这里报**国内版**那一家的条数 —— 与旧口径对齐（同一时刻的同一份清单）。
        // 完整的可用清单看 `/v1/models`（聚合视图）。
        "models": crate::server::core::providers::catalog::manifest_for(
            crate::server::core::providers::ProviderKind::WorkBuddy,
        )
        .len(),
    }))
}
