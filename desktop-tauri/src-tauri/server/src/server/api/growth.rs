//! WorkBuddy **成长任务**的管理端点（查询 / 执行 / 开学季 / 猫猫旅行）。
//!
//! ```text
//!   GET  /api/accounts/{id}/growth          任务盘点（只读）：成长任务 + 能量 +
//!                                           旅行状态 + 开学季概况
//!   POST /api/accounts/{id}/growth/task     执行**单个**任务 { code, mode: full|claim }
//!   POST /api/accounts/{id}/growth/school   开学季任务 + 可选抽奖 { only?, lottery? }
//!   POST /api/accounts/{id}/growth/travel   猫猫旅行巡检一次
//! ```
//!
//! ── 这里只做转发 ────────────────────────────────────────────
//! 执行体在**提供商插件**里（`workbuddy::admin`）：任务树 / 开学季 / 猫猫旅行
//! 是 WorkBuddy 独有的运营玩法，与「怎么伪装成官方客户端打上游」是同一类
//! 知识 —— 按项目的风险隔离要求，这类实现不留在本仓库。网关这边只剩下
//! HTTP 形状（鉴权、账号归属校验、信封），动作名与返回 JSON 由插件定义并经
//! `admin.action` 透传。
//!
//! ── 为什么执行是「单任务」而不是「全量一批」─────────────────
//! 全量执行（14+ 任务）必须逐条留写动作间隔（参考项目实测的风控口径 ≥1s），
//! 总时长可达一两分钟 —— 而桌面壳的管理 API 转发有 60 秒单请求超时
//! （`gateway::REQUEST_TIMEOUT_MS`），一次全量请求会被中途掐断且不留痕迹。
//! 所以这里只暴露**单任务**执行端点：界面的「一键完成」在本地循环逐条调用
//! （每次请求都是几秒级），既能显示逐条进度、也天然不受总超时约束 ——
//! 与批量签到「一次请求跑完」的形态不同，那是因为签到的单账号动作远短于任务链。
//!
//! ── 只服务 WorkBuddy 国内版 ─────────────────────────────────
//! 门控在插件里（`supports_growth` 只认国内版 provider id）：国际版没有 CN
//! 任务中心，请求会拿到 400 与可读原因 —— 与单账号签到路径「明确报错、
//! 不静默跳过」的纪律一致。
//!
//! ── 写动作的用户意图 ────────────────────────────────────────
//! 与参考脚本（默认 dry-run、`--yes` 才写）不同：这里的执行端点由用户
//! 在界面上点击触发，点击即意图。但**查询与执行分开两条端点**：页面展开
//! 只查（`GET`）、按钮才是写（`POST`），不给「看一眼就改状态」留机会。

use serde_json::Value;

use axum::body::Bytes;
use axum::response::Response;

use crate::server::errors::management_error;
use crate::server::http::ok_json;
use crate::server::ServerState;

/// 解析请求体（缺省 / 非 JSON 一律给空对象）。
fn parse_body(body: &Bytes) -> Value {
    serde_json::from_slice(body).unwrap_or(Value::Null)
}

/// 取账号记录（找不到报 404）。
fn account_of(state: &ServerState, account_id: &str) -> Result<Value, Response> {
    let accounts = state
        .store()
        .list_accounts()
        .get("accounts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    accounts
        .into_iter()
        .find(|account| account.get("id").and_then(Value::as_str) == Some(account_id))
        .ok_or_else(|| management_error(404, "账号不存在"))
}

/// 调一次管理动作（账号归属校验 + 插件调用）。
///
/// 归属校验留在网关：`admin.action` 只会打到**该 provider 的插件进程**，
/// 拿错家的账号 id 打过去，插件读不到记录（500/401），用户看到的是一句
/// 无关的底层错误。这里先按账号记录确认它确实属于本家，再发 RPC。
async fn call_action(
    state: &ServerState,
    account_id: &str,
    action: &str,
    payload: &Value,
) -> Result<Value, Response> {
    let account = account_of(state, account_id)?;
    let provider = account
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if !crate::server::core::plugin_host::serves_provider(&provider) {
        return Err(management_error(
            400,
            "该账号的提供商没有安装插件，无法执行成长任务",
        ));
    }
    let Some(adapter) = crate::server::core::providers::adapter::adapter_by_id(&provider) else {
        return Err(management_error(400, "该账号的提供商未注册"));
    };
    let _ = account;
    adapter
        .admin_action(state.store(), account_id, action, payload)
        .await
        .map_err(|error| management_error(error.status_code, error.message))
}

/// `GET /api/accounts/{id}/growth` —— 任务盘点（只读，不发任何写请求）。
///
/// 返回四块：成长任务清单（含每任务的展示与支持标记）、能量/连登读数、
/// 猫猫旅行状态、开学季概况（含抽奖余额）。各块相互独立：某一块拉取失败
/// 只在该块给 `error` 字段，不影响其余（用户要看到「哪块坏了」）。
pub async fn status(state: &ServerState, account_id: &str) -> Response {
    match call_action(state, account_id, "growth.status", &Value::Null).await {
        Ok(data) => ok_json(data),
        Err(response) => response,
    }
}

/// `POST /api/accounts/{id}/growth/task` —— 执行**单个**任务。
///
/// `mode=full`（缺省）：accept（若未接单）→ 点亮上报 → 回读 → claim；
/// `mode=claim`：只领奖（该任务已完成/已领时执行 claim，幂等；未完成跳过）。
///
/// 返回单条结果行（code / outcome / message / credit / energy）。界面的
/// 「一键完成」在本地循环逐条调用本端点（见模块头的 60 秒超时说明）。
pub async fn run_task(state: &ServerState, account_id: &str, body: &Bytes) -> Response {
    let payload = parse_body(body);
    match call_action(state, account_id, "growth.task", &payload).await {
        Ok(data) => ok_json(data),
        Err(response) => response,
    }
}

/// `POST /api/accounts/{id}/growth/school` —— 开学季任务 + 可选抽奖。
///
/// `only` 限定任务码；`lottery=true` 时任务之后把抽奖余额抽空（上限 20 次护栏）。
pub async fn school_run(state: &ServerState, account_id: &str, body: &Bytes) -> Response {
    let payload = parse_body(body);
    match call_action(state, account_id, "growth.school", &payload).await {
        Ok(data) => ok_json(data),
        Err(response) => response,
    }
}

/// `POST /api/accounts/{id}/growth/travel` —— 猫猫旅行巡检一次。
///
/// 一趟只推进一步（无猫 → 领养；idle → 派出；arrived → 领奖；traveling → 跳过）。
pub async fn travel_run(state: &ServerState, account_id: &str) -> Response {
    match call_action(state, account_id, "growth.travel", &Value::Null).await {
        Ok(data) => ok_json(data),
        Err(response) => response,
    }
}

/// 供 `api::checkin_center` 之类只想知道「这家有没有成长任务」的地方使用：
/// 只有国内版 WorkBuddy 有。
pub fn supports_growth(account: &Value) -> bool {
    account.get("provider").and_then(Value::as_str) == Some("workbuddy")
        && !account
            .get("enterpriseId")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty())
}
