//! Qoder UMID 组件（设备风控身份）的状态与安装：
//!
//! - `GET  /api/qoder-umid` → 插件动作 `risk.status`（available / source /
//!   installSupported / installing）
//! - `POST /api/qoder-umid/install` → 插件动作 `risk.install`（下载安装；仅
//!   Linux；Windows / macOS 走本机客户端 / qodercli，无需下载）
//!
//! 为什么是独立一条而不是塞进账号路由：组件是**机器级**的（一次安装服务全部
//! 国际版账号），与任何一条账号记录无关；塞进 `/api/accounts/{id}/…` 会让
//! 「组件在不在」看起来像账号的状态。
//!
//! ── 实现已整体搬到提供商插件 ─────────────────────────────────
//! 组件从哪个 npm 包里取、怎么校验 integrity、ELF 按哪个字段挑架构、装到
//! 哪个路径 —— 全是「怎么让上游认为这是官方客户端」的知识，按项目的风险隔离
//! 要求不留在本仓库。网关这边只剩「把界面的一次点击转成一次 RPC」。
//!
//! 消费方：签到中心在 Qoder 国际版分组的说明里展示组件状态，Linux 且未安装
//! 时给「一键安装」按钮（安装耗时以分钟计 —— 要下载 ~10MB 的 npm 包并解出
//! 二进制，前端按下后轮询 GET 看 installing 即可）。

use axum::extract::State;
use axum::response::Response;
use serde_json::Value;

use crate::server::errors::management_error;
use crate::server::http::ok_json;
use crate::server::ServerState;

/// 组件状态属于**国际版**（`qoder-intl`）那一家的插件实例。
const PROVIDER: &str = "qoder-intl";

/// 把一次界面动作转成插件的 `admin.action`。
async fn call_action(state: &ServerState, action: &str) -> Result<Value, (i32, String)> {
    let Some(adapter) = crate::server::core::providers::adapter::adapter_by_id(PROVIDER) else {
        return Err((400, "Qoder 的提供商插件没有安装，无法查询设备组件".to_string()));
    };
    adapter
        .admin_action(state.store(), "", action, &Value::Null)
        .await
        .map_err(|error| (error.status_code, error.message))
}

/// GET /api/qoder-umid
pub async fn get_status(State(state): State<ServerState>) -> Response {
    match call_action(&state, "risk.status").await {
        Ok(status) => ok_json(status),
        Err((status_code, message)) => management_error(status_code, message),
    }
}

/// POST /api/qoder-umid/install
pub async fn install(State(state): State<ServerState>) -> Response {
    match call_action(&state, "risk.install").await {
        Ok(manifest) => {
            crate::server::logging::log("[Qoder]", "UMID 组件安装完成（来自签到中心）");
            ok_json(manifest)
        }
        Err((status_code, message)) => management_error(status_code, message),
    }
}
