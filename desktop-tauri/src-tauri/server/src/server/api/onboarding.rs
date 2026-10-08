//! Loomy 新手任务的两个管理端点（查询 / 领取）。
//!
//! ```text
//!   GET  /api/accounts/{id}/onboarding        任务状态（只读）
//!   POST /api/accounts/{id}/onboarding/claim  一键领取全部未完成任务
//! ```
//!
//! ── 为什么查询是 GET、领取是 POST ───────────────────────────
//! 与本仓「账号 + 子动作」后缀路由的既有形状一致：读走 GET（可收藏、可重放、
//! 无副作用），写走 POST（`onboarding/claim` 不以 `/onboarding` 结尾，两条
//! 后缀互不包含，分派顺序无所谓）。分派接线在 `api::accounts::dispatch`
//! 的 GET 后缀段与 POST 后缀段各一条。
//!
//! ── 与签到的关系（为什么不在 checkin 里顺手领）──────────────
//! 签到是每日动作且与定时签到共用同一段执行体（`core::billing::checkin`），
//! 把一次性的新手任务塞进去，会让之后每一轮定时签到都白打查询接口。界面侧
//! 在签到完成后自己查一次、有未领取才弹窗（见 ui-islands 的
//! accounts-dialog-onboarding），领完自然不再弹。
//!
//! 响应形状（core::providers::loomy::onboarding）：
//!   · status  → `{tasks:[{key,title,group,points,done}], earned, total, unclaimed}`
//!   · claim   → `{results:[{key,ok,already?,error?}], claimed, failed,
//!                 claimedPoints, tasks, earned, total, unclaimed}`

use axum::response::Response;

use crate::server::core::providers::loomy::onboarding;
use crate::server::errors::management_error;
use crate::server::http::ok_json;
use crate::server::ServerState;

/// `GET /api/accounts/{id}/onboarding` —— 任务状态快照（不发任何写请求）。
pub async fn status(state: &ServerState, account_id: &str) -> Response {
    match onboarding::get_tasks(state.store(), account_id).await {
        Ok(data) => ok_json(data),
        Err(error) => management_error(error.status_code, error.message),
    }
}

/// `POST /api/accounts/{id}/onboarding/claim` —— 串行上报全部未完成的 key。
/// 已完成的 key 不发请求，重复调用天然幂等。
pub async fn claim(state: &ServerState, account_id: &str) -> Response {
    match onboarding::claim_all(state.store(), account_id).await {
        Ok(data) => ok_json(data),
        Err(error) => management_error(error.status_code, error.message),
    }
}
