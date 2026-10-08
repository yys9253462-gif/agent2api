//! 签到中心的聚合快照（GET /api/checkin-center）。
//!
//! ── 为什么单独一条聚合端点 ──────────────────────────────────
//! 签到中心一页要看四块数据：每日签到的账号分组、自动签到设置、签到历史台账、
//! 一次性/手动项（新手任务 / 福利 / 套餐）的账号清单。前两块已有各自接口
//! （/api/accounts、/api/auto-checkin），但「按提供商分组 + 支持签到判定 +
//! 今日已签」的口径在后端（`supports_checkin` / `CHECKIN_PROVIDERS`），前端拼
//! 会抄一份判定逻辑 —— 那正是「两套行为」的开头。所以这里在服务端聚合成一份，
//! 一次请求拉全，页面打开只打这一条。
//!
//! 惰性查询的**上游资格类状态**（Loomy 新手任务清单、CodeArts 福利资格、
//! ZCode 可领套餐）**不在**这份快照里：它们每查一个账号就要打一次上游，
//! 聚合快照必须保持「打开页面零上游请求」。快照只给账号入口清单，前端拿到
//! 清单后按需调既有的查询接口（/onboarding、/codearts-welfare/preview、
//! /zcode-claim/preview）—— 那三条接口原样复用，一条不改。
//!
//! ── 分组口径与批量签到完全一致 ─────────────────────────────
//! 进「每日签到」分组的账号 = 提供商 ∈ `CHECKIN_PROVIDERS` **且**
//! `supports_checkin` 为真 —— 与 `resolve_checkin_targets` 的过滤是同一对判据，
//! 签到中心看到的「可签 9 个」与点「立即全部签到」实际签到的集合不会分叉。
//! 其余账号进 `outOfScope`（按提供商聚合，附原因文案）。
//!
//! ── 响应形状 ───────────────────────────────────────────────
//! ```json
//! {
//!   "daily": {
//!     "providers": [ { "id", "label",
//!                      "accounts": [ { "id", "name", "available", "checkinAt",
//!                                      "checkedInToday" } ],
//!                      "doneCount", "totalCount" } ],
//!     "outOfScope": [ { "label", "reason", "count" } ],
//!     "todayDone": 6, "todayEligible": 9
//!   },
//!   "extras": {
//!     "onboarding": [ { "id", "name" } ],
//!     "welfare":    [ { "id", "name" } ],
//!     "plans":      [ { "id", "name", "claimAt" } ]
//!   },
//!   "auto": { …与 GET /api/auto-checkin 同形… },
//!   "history": [ { "at", "date", "reason", "succeeded", "total", "skipped",
//!                  "failed", "failedCount" } ]
//! }
//! ```
//! `history` 新条目在前（`checkin_history` 的落盘顺序），前端直接按序渲染时间线。

use std::collections::BTreeMap;

use axum::extract::State;
use axum::response::Response;
use chrono::DateTime;
use serde_json::{json, Value};

use crate::server::core::auto_checkin::{self, CHECKIN_PROVIDERS};
use crate::server::core::billing::checkin;
use crate::server::core::checkin_history;
use crate::server::http::ok_json;
use crate::server::ServerState;

/// `checkedInToday` 的判定：`checkinAt`（ms）落在今天（本地时区）。
/// 时间戳缺失 / 非法都算「今天没签」—— 与账号页「checkinAt 落在今天即算签过」
/// 的口径一致（见 store_admin::mark_checkin 的说明）。
fn checked_in_today(checkin_at: Option<i64>) -> bool {
    let Some(ms) = checkin_at else {
        return false;
    };
    let Some(at) = DateTime::from_timestamp_millis(ms) else {
        return false;
    };
    auto_checkin::local_date_key(at.with_timezone(&chrono::Local))
        == auto_checkin::local_date_key(chrono::Local::now())
}

/// 账号快照里的字段（缺失按空值兜底，与 billing::checkin 的读取口径一致）
fn account_text(account: &Value, key: &str) -> String {
    account
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

/// outOfScope 的原因文案（按 `supports_checkin` 的排除分支逐一对应）：
/// 判据改了这里要跟着改 —— 两处写的都是「这个账号为什么不能签到」。
fn out_of_scope_reason(account: &Value) -> &'static str {
    if account.get("edition").and_then(Value::as_str) == Some("intl") {
        return "国际版账号没有签到活动";
    }
    let provider = account_text(account, "provider");
    if crate::server::core::account_store::is_accio_family(&provider) {
        return "未接入签到链路";
    }
    "没有签到链路"
}

/// 签到中心的聚合快照（不发任何上游请求 —— 资格类状态由前端按需惰性查询）
pub async fn get_center(State(state): State<ServerState>) -> Response {
    let accounts: Vec<Value> = state
        .store()
        .list_accounts()
        .get("accounts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    // ── 每日签到分组：提供商 ∈ CHECKIN_PROVIDERS 且 supports_checkin ──
    // 按 CHECKIN_PROVIDERS 的注册顺序分桶（界面行序稳定）；范围外账号按
    // 「提供商 + 原因」聚合（label 用注册表名，理由取排除分支的文案）。
    let mut buckets: Vec<Vec<Value>> = vec![Vec::new(); CHECKIN_PROVIDERS.len()];
    let mut out_of_scope: BTreeMap<String, (String, String, i64)> = BTreeMap::new();
    for account in &accounts {
        let provider = account_text(account, "provider");
        let provider = if provider.is_empty() {
            crate::server::core::providers::DEFAULT_PROVIDER_ID.to_string()
        } else {
            provider
        };
        let slot = CHECKIN_PROVIDERS
            .iter()
            .position(|id| *id == provider)
            .filter(|_| checkin::supports_checkin(account));
        if let Some(index) = slot {
            buckets[index].push(json!({
                "id": account_text(account, "id"),
                "name": Value::from(account_text(account, "name")),
                "available": account.get("available").and_then(Value::as_bool).unwrap_or(true),
                "checkinAt": account.get("checkinAt").cloned().unwrap_or(Value::Null),
                "checkedInToday": checked_in_today(
                    account.get("checkinAt").and_then(Value::as_i64),
                ),
            }));
        } else {
            let label = crate::server::core::providers::label_of(&provider);
            let reason = out_of_scope_reason(account);
            let entry = out_of_scope
                .entry(format!("{provider}|{reason}"))
                .or_insert_with(|| (label, reason.to_string(), 0));
            entry.2 += 1;
        }
    }

    let provider_rows: Vec<Value> = CHECKIN_PROVIDERS
        .iter()
        .zip(buckets.into_iter())
        .map(|(id, rows)| {
            let done = rows
                .iter()
                .filter(|row| row.get("checkedInToday").and_then(Value::as_bool) == Some(true))
                .count();
            json!({
                "id": id,
                "label": auto_checkin::provider_label(id),
                "accounts": rows,
                "doneCount": done,
                "totalCount": rows.len(),
            })
        })
        .collect();
    let today_done: u64 = provider_rows
        .iter()
        .map(|row| row.get("doneCount").and_then(Value::as_u64).unwrap_or(0))
        .sum();
    let today_eligible: u64 = provider_rows
        .iter()
        .map(|row| row.get("totalCount").and_then(Value::as_u64).unwrap_or(0))
        .sum();
    let out_rows: Vec<Value> = out_of_scope
        .into_values()
        .map(|(label, reason, count)| json!({ "label": label, "reason": reason, "count": count }))
        .collect();

    // ── 一次性 / 手动项的账号入口清单（资格状态由前端惰性查询）──
    // 三家的账号行都是 {id, name}；ZCode 额外带公开的 `claimAt`（上次领取时间，
    // 与 checkinAt 同一处置：给原始时间戳，不给布尔，见 zcode_accounts 的说明）。
    let extra_rows = |provider: &str, with_claim_at: bool| -> Vec<Value> {
        accounts
            .iter()
            .filter(|account| account_text(account, "provider") == provider)
            .map(|account| {
                let mut row = json!({
                    "id": account_text(account, "id"),
                    "name": Value::from(account_text(account, "name")),
                });
                if with_claim_at {
                    if let Some(map) = row.as_object_mut() {
                        map.insert(
                            "claimAt".to_string(),
                            account.get("claimAt").cloned().unwrap_or(Value::Null),
                        );
                    }
                }
                row
            })
            .collect()
    };
    let onboarding_rows = extra_rows("loomy", false);
    let welfare_rows = extra_rows("codearts", false);
    let plan_rows: Vec<Value> = accounts
        .iter()
        .filter(|account| {
            let provider = account_text(account, "provider");
            provider == "zcode" || provider == "zcode-intl"
        })
        .map(|account| {
            json!({
                "id": account_text(account, "id"),
                "name": Value::from(account_text(account, "name")),
                "claimAt": account.get("claimAt").cloned().unwrap_or(Value::Null),
                // 领取台账（{planId: 毫秒}）原样带出：界面「今天领过哪几份」按
                // 逐份比对（claimPlans 的日界判定在前端已有同款实现，复用它，
                // 后端不再抄一份北京时间的日界逻辑）
                "claimPlans": account.get("claimPlans").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();

    ok_json(json!({
        "daily": {
            "providers": provider_rows,
            "outOfScope": out_rows,
            "todayDone": today_done,
            "todayEligible": today_eligible,
        },
        "extras": {
            "onboarding": onboarding_rows,
            "welfare": welfare_rows,
            "plans": plan_rows,
        },
        "auto": state.auto_checkin().state(),
        "history": checkin_history::list(),
    }))
}
