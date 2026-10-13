//! AutoClaw 的**活动福利到账确认**（Weekly Perks 这类「活跃即发」活动）。
//!
//! ── 上游是什么（从客户端产物 + 实测确认，2026-10-13）──────────
//! 这类活动**没有领取接口**：服务端在用户「登录活跃」时自动入账。实测证据：
//!
//!   ```text
//!   2026-10-13 16:18:46  桌面端打开 → token 刷新（JWT iat 同时刻）
//!   2026-10-13 16:18:48  流水 +800「AutoClaw Weekly Perks +800 credits」
//!   2026-10-13 16:19:20  每日签到 +400（task-complete，与活动福利无关）
//!   ```
//!
//! 活动入账的流水元数据带 `activity_id: autoclaw_user_active_reward_*`、
//! `grant_date`（北京日期）与 `source: campaign`；「登录」的判据是服务端侧
//! 的 `last_login_at`（只在登录 / token 刷新时更新）—— 普通只读接口与
//! `daily_signin` 的 task-complete **都不更新它**（逐条实测过），因此
//! **签到链本身不会触发活动发放**。
//!
//! ── 本模块只做「确认」；「触发」在 `checkin.rs` ────────────────
//! 缺少的领取动作由签到链在 `claim_daily_signin` 里补：今天没到账就先刷新一次
//! 登录态（`refresh::refresh` 的 force 通道）—— 那条路径的安全性依据
//!（服务端虽轮换 refresh_token、但旧令牌可重复使用，2026-10-13 实测）记在
//! `checkin.rs` 的模块头。本模块只管读积分流水回答「今天的活动福利到账了吗」。
//!
//! ── 查询失败不影响签到结论 ──────────────────────────────────
//! 与 `balance.rs` 里「订阅失败不影响积分结果」同一取向：活动福利是附加
//! 信息，查不到（网络抖 / 活动结束）就给 `null`，绝不把它升级成签到失败。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：零 unwrap/expect/panic。

use serde_json::{json, Value};

use crate::server::core::account_store::AccountStore;
use crate::server::core::beijing;
use crate::server::errors::GatewayError;

use super::balance::userapi_get;
use super::credentials;
use super::region::Region;

/// 积分流水路径（只取**入账**方向：模型消费的 debit 不会挤占窗口）。
///
/// `flow_direction` 的合法值是 `income`（实测：`credit` / `in` 都是 400001）。
const LEDGERS_PATH: &str = "/agent-assetmgr/api/v1/ledgers_std?asset_type=point&wallet_type=all&cursor=&page_size=30&flow_direction=income";

/// 活动福利的 `activity_id` 前缀。
///
/// 当前值 `autoclaw_user_active_reward_202609`（202609 = 2026-09 创建的活动），
/// 服务端换版本时会改后缀 —— 前缀匹配以兼容；这个活动从 9 月起就是「活跃即发」
/// 的同一套机制（9-22 的「Credits Boost Station」也是它的一个档位）。
const ACTIVE_REWARD_ACTIVITY_PREFIX: &str = "autoclaw_user_active_reward";

/// 今天的活动福利到账状态。
///
/// 返回形状（`promotionReward` 字段的值）：
///   - 到账：`{"granted": true, "points": 800, "description": "...", "grantDate": "YYYY-MM-DD"}`
///   - 未到账：`{"granted": false, "grantDate": "YYYY-MM-DD"}`
///
/// 判据是**流水里存在今天的**活动福利入账：`metadata.grant_date` 等于北京
/// 今天的日期键（`core::beijing`，与签到链路同一个「今天」口径）且
/// `metadata.activity_id` 命中前缀。
pub async fn today_reward(
    region: Region,
    store: &AccountStore,
    account_id: &str,
) -> Result<Value, GatewayError> {
    let record = store.autoclaw_account_record(region, account_id);
    if !account_id.is_empty() && record.is_none() {
        return Err(GatewayError::with_status(
            404,
            format!(
                "AutoClaw {}账号 {account_id} 不存在或不属于该地区",
                region.label()
            ),
        ));
    }
    let credentials = credentials::snapshot_for(record.as_ref(), region)?;
    if credentials.token.trim().is_empty() {
        return Err(GatewayError::with_status(401, "该账号没有可用凭证，无法查询活动福利"));
    }

    let payload = userapi_get(&credentials, LEDGERS_PATH, "活动福利查询").await?;
    let today = beijing::today_key();

    let entry = payload
        .get("data")
        .and_then(|data| data.get("entries"))
        .and_then(Value::as_array)
        .and_then(|entries| entries.iter().find(|entry| is_today_reward(entry, &today)));

    let Some(entry) = entry else {
        return Ok(json!({ "granted": false, "grantDate": today }));
    };
    Ok(json!({
        "granted": true,
        "points": entry.get("amount").and_then(Value::as_i64).unwrap_or(0),
        "description": entry
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
        "grantDate": today,
    }))
}

/// 这条流水是不是「今天的活动福利」。
fn is_today_reward(entry: &Value, today: &str) -> bool {
    let Some(metadata) = entry.get("metadata") else {
        return false;
    };
    let grant_date = metadata.get("grant_date").and_then(Value::as_str).unwrap_or("");
    if grant_date != today {
        return false;
    }
    metadata
        .get("activity_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .starts_with(ACTIVE_REWARD_ACTIVITY_PREFIX)
}
