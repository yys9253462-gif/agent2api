//! 新手任务的两个管理端点（查询 / 领取），按账号的提供商分派执行体。
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
//! ── 按提供商分派 ────────────────────────────────────────────
//! 新手任务目前两家有：Loomy（任务表 + 逐项上报，`loomy::onboarding`）与
//! 小浣熊（首次登录奖励双任务探测 —— 电脑端 / 手机端各一条端点，
//! `raccoon::onboarding`）。分派键是账号记录里的 provider 字段；查不到记录或
//! 缺省一律走 Loomy —— 与既有行为一致（`loomy_account_record` 会对别家 id 报
//! 404，兜底语义不变）。
//!
//! ── 与签到的关系（为什么不在 checkin 里顺手领）──────────────
//! 签到是每日动作且与定时签到共用同一段执行体（`core::billing::checkin`），
//! 把一次性的新手任务塞进去，会让之后每一轮定时签到都白打查询接口。界面侧
//! 在签到完成后自己查一次、有未领取才自动领取（见 ui-islands 的
//! checkin-state），领完自然不再弹。
//!
//! 响应形状（两家对齐）：
//!   · status  → `{tasks:[{key,title,group,points,done}], earned, total, unclaimed}`
//!   · claim   → `{results:[{key,ok,already?,error?}], claimed, failed,
//!                 claimedPoints, tasks, earned, total, unclaimed}`

use serde_json::Value;

use axum::response::Response;

use crate::server::core::providers::{loomy::onboarding as loomy_onboarding, raccoon::onboarding as raccoon_onboarding};
use crate::server::errors::management_error;
use crate::server::http::ok_json;
use crate::server::ServerState;

/// 小浣熊（raccoon）新手任务的分派键（`providers::ProviderMeta` 的 id）
const RACCOON_PROVIDER_ID: &str = "raccoon";

/// 从账号存储里查一条账号的 provider id（查不到账号给空串 —— 分派会落到
/// Loomy 分支，由 `loomy_account_record` 报 404，与既有兜底行为一致）。
fn provider_of_account(state: &ServerState, account_id: &str) -> String {
    state
        .store()
        .list_accounts()
        .get("accounts")
        .and_then(Value::as_array)
        .and_then(|accounts| {
            accounts
                .iter()
                .find(|account| account.get("id").and_then(Value::as_str) == Some(account_id))
                .and_then(|account| account.get("provider").and_then(Value::as_str))
        })
        .unwrap_or("")
        .to_string()
}

/// `GET /api/accounts/{id}/onboarding` —— 任务状态快照。
pub async fn status(state: &ServerState, account_id: &str) -> Response {
    let result = if provider_of_account(state, account_id) == RACCOON_PROVIDER_ID {
        raccoon_onboarding::get_tasks(state.store(), account_id).await
    } else {
        loomy_onboarding::get_tasks(state.store(), account_id).await
    };
    match result {
        Ok(data) => ok_json(data),
        Err(error) => management_error(error.status_code, error.message),
    }
}

/// `POST /api/accounts/{id}/onboarding/claim` —— 领取全部未完成任务。
/// 两家执行体自身都幂等，重复调用不会重复加分。
pub async fn claim(state: &ServerState, account_id: &str) -> Response {
    let result = if provider_of_account(state, account_id) == RACCOON_PROVIDER_ID {
        raccoon_onboarding::claim_all(state.store(), account_id).await
    } else {
        loomy_onboarding::claim_all(state.store(), account_id).await
    };
    match result {
        Ok(data) => ok_json(data),
        Err(error) => management_error(error.status_code, error.message),
    }
}
