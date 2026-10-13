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
//! ── 这是什么 ────────────────────────────────────────────────
//! 从参考项目 workbuddy2api 的 scripts/task_runner.py 与 school_open_day_2026.py
//! 移植的「成长任务」能力：查询任务树 → 接单(accept) → 点亮(事件上报) →
//! 领奖(claim)。上游把产品行为做成一棵任务树（建画布、用专家、发对话……），
//! 完成任务送 Credit 与 Energy。执行体在
//! `core::providers::workbuddy::growth`（含 school / travel 子域）。
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
//! 判据在 `growth::supports_growth`：国际版没有 CN 任务中心（参考项目
//! `auth_is_global` 同款结论），对它的请求报 400 并给出可读原因 —— 与
//! 单账号签到路径「明确报错、不静默跳过」的纪律一致。
//!
//! ── 写动作的用户意图 ────────────────────────────────────────
//! 与参考脚本（默认 dry-run、`--yes` 才写）不同：这里的执行端点由用户
//! 在界面上点击触发，点击即意图。但**查询与执行分开两条端点**：页面展开
//! 只查（`GET`）、按钮才是写（`POST`），不给「看一眼就改状态」留机会。

use serde_json::{json, Value};

use axum::body::Bytes;
use axum::response::Response;

use crate::server::core::providers::workbuddy::growth::{
    self, school, spec, travel, GrowthAccount, TaskOutcome,
};
use crate::server::errors::management_error;
use crate::server::http::ok_json;
use crate::server::ServerState;

/// 解析请求体里的 `{ code, mode, only, lottery }`（缺省 / 非 JSON 都给默认值）。
struct GrowthRequest {
    code: String,
    mode: String,
    only: Vec<String>,
    lottery: bool,
}

fn parse_request(body: &Bytes) -> GrowthRequest {
    let payload: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let code = payload
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let mode = payload
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("full")
        .to_string();
    let only = payload
        .get("only")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let lottery = payload.get("lottery").and_then(Value::as_bool).unwrap_or(false);
    GrowthRequest { code, mode, only, lottery }
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

/// 取 growth 域账号上下文（国内版校验 + 凭证派生）。
fn growth_account(state: &ServerState, account_id: &str) -> Result<GrowthAccount, Response> {
    let account = account_of(state, account_id)?;
    if !growth::supports_growth(&account) {
        return Err(management_error(
            400,
            "该账号不支持成长任务（仅 WorkBuddy 国内版）",
        ));
    }
    GrowthAccount::from_store(state.store(), account_id)
        .ok_or_else(|| management_error(400, "账号没有可用凭证，请先刷新登录态"))
}

/// 任务行的展示形状（界面按它渲染任务清单）。
fn task_row(task: &Value) -> Value {
    let code = task.get("task_code").and_then(Value::as_str).unwrap_or("");
    let spec = spec::spec_of(code);
    let progress = task.get("progress").cloned().unwrap_or(Value::Null);
    json!({
        "code": code,
        "title": task.get("title").and_then(Value::as_str).unwrap_or(""),
        "description": task.get("description").and_then(Value::as_str).unwrap_or(""),
        "status": task.get("accept_status").and_then(Value::as_str).unwrap_or(""),
        "current": progress.get("current").and_then(Value::as_i64).unwrap_or(0),
        "target": progress.get("target").and_then(Value::as_i64).unwrap_or(0),
        "locked": task.get("locked").and_then(Value::as_bool).unwrap_or(false),
        "unlockAt": task.get("unlock_at").cloned().unwrap_or(Value::Null),
        // 是否在映射表里（未映射任务在执行时会保守跳过 —— 界面如实标注）
        "supported": spec.is_some(),
        "unforgeable": spec.as_ref().map(|spec| spec.unforgeable).unwrap_or(false),
        "unforgeableReason": spec.as_ref().map(|spec| spec.reason).unwrap_or(""),
        "kind": spec.as_ref().map(|spec| spec.kind).unwrap_or(""),
        "source": spec.as_ref().map(|spec| spec.src).unwrap_or(""),
    })
}

/// `GET /api/accounts/{id}/growth` —— 任务盘点（只读，不发任何写请求）。
///
/// 返回四块：成长任务清单（含每任务的展示与支持标记）、能量/连登读数、
/// 猫猫旅行状态、开学季概况（含抽奖余额）。各块相互独立：某一块拉取失败
/// 只在该块给 `error` 字段，不影响其余（用户要看到「哪块坏了」）。
pub async fn status(state: &ServerState, account_id: &str) -> Response {
    let account = match growth_account(state, account_id) {
        Ok(account) => account,
        Err(response) => return response,
    };

    // ── 成长任务清单 ──
    // 两个口径都要拉：小程序限定任务（Sequential_Tasks_*）只在带
    // `X-Client-Platform: miniprogram` 的列表里下发，默认口径看不到它们。
    // 合并时默认口径优先（同码不重复），mp 任务按 code 去重追加。
    let tasks_block = match growth::list_tasks(&account, false).await {
        Ok(default_tasks) => {
            let mut rows: Vec<Value> = default_tasks.iter().map(task_row).collect();
            let mut known: Vec<String> = rows
                .iter()
                .filter_map(|row| row.get("code").and_then(Value::as_str))
                .map(str::to_string)
                .collect();
            if let Ok(mp_tasks) = growth::list_tasks(&account, true).await {
                for task in &mp_tasks {
                    let code = task.get("task_code").and_then(Value::as_str).unwrap_or("");
                    if code.is_empty() || known.iter().any(|item| item == code) {
                        continue;
                    }
                    known.push(code.to_string());
                    rows.push(task_row(task));
                }
            }
            json!({
                "tasks": rows,
                "error": Value::Null,
            })
        }
        Err(error) => json!({ "tasks": [], "error": error }),
    };

    // ── 能量 / 连登读数（只读 oracle，失败给 null）──
    let energy = growth::energy_balance(&account).await;
    let streak = growth::growth_streak(&account).await;

    // ── 猫猫旅行状态 ──
    let travel_block = match travel::buddy_info(&account).await {
        Ok(buddy) => {
            let status = travel::travel_status(&account).await;
            match (buddy, status) {
                (Some(buddy), Ok(status)) => json!({
                    "hasBuddy": true,
                    "buddyName": buddy.name,
                    "state": status.state,
                    "dailyLimitReached": status.daily_limit_reached,
                    "recordId": status.record_id,
                    "rewardCredit": status.reward_credit,
                    "error": Value::Null,
                }),
                (Some(buddy), Err(error)) => json!({
                    "hasBuddy": true,
                    "buddyName": buddy.name,
                    "state": "",
                    "error": error,
                }),
                (None, _) => json!({
                    "hasBuddy": false,
                    "buddyName": "",
                    "state": "",
                    "error": Value::Null,
                }),
            }
        }
        Err(error) => json!({ "hasBuddy": Value::Null, "error": error }),
    };

    // ── 开学季概况（任务列表 + 抽奖余额）──
    let school_block = match school::fetch_tasks(&account).await {
        Ok((tasks, in_period)) => {
            let lottery = school::fetch_lottery(&account).await;
            let chance = lottery
                .as_ref()
                .ok()
                .map(|(_, chance)| {
                    json!({
                        "balance": chance.get("balance").and_then(Value::as_i64).unwrap_or(0),
                        "totalEarned": chance.get("total_earned").and_then(Value::as_i64).unwrap_or(0),
                    })
                })
                .unwrap_or(Value::Null);
            json!({
                "inPeriod": in_period,
                "tasks": tasks.iter().map(|task| {
                    let code = task.get("task_code").and_then(Value::as_str).unwrap_or("");
                    json!({
                        "code": code,
                        "title": task.get("title").and_then(Value::as_str).unwrap_or(""),
                        "status": task.get("status").and_then(Value::as_str).unwrap_or(""),
                        "current": task.get("progress").and_then(Value::as_i64).unwrap_or(0),
                        "target": task.get("target_count").and_then(Value::as_i64).unwrap_or(0),
                        "rewardCredit": task.get("reward_credit").and_then(Value::as_i64).unwrap_or(0),
                    })
                }).collect::<Vec<Value>>(),
                "lottery": chance,
                "error": Value::Null,
            })
        }
        Err(error) => json!({ "inPeriod": false, "tasks": [], "lottery": Value::Null, "error": error }),
    };

    ok_json(json!({
        "id": account_id,
        "uid": account.uid,
        "nick": account.nick,
        "energy": energy,
        "streak": streak,
        "tasks": tasks_block,
        "travel": travel_block,
        "school": school_block,
    }))
}

/// `POST /api/accounts/{id}/growth/task` —— 执行**单个**任务。
///
/// `mode=full`（缺省）：accept（若未接单）→ 点亮上报 → 回读 → claim；
/// `mode=claim`：只领奖（该任务已完成/已领时执行 claim，幂等；未完成跳过）。
///
/// 返回单条结果行（code / outcome / message / credit / energy）。界面的
/// 「一键完成」在本地循环逐条调用本端点（见模块头的 60 秒超时说明）。
pub async fn run_task(state: &ServerState, account_id: &str, body: &Bytes) -> Response {
    let account = match growth_account(state, account_id) {
        Ok(account) => account,
        Err(response) => return response,
    };
    let request = parse_request(body);
    if request.code.is_empty() {
        return management_error(400, "缺少任务码（code）");
    }
    // 拉当前任务状态（写之前先看现状：已领的走幂等 claim，未接单的先 accept）。
    // 列表口径按任务的域走：小程序任务只在带 mp 头的列表里下发（见 status 的说明）。
    let mp = spec::spec_of(&request.code).map(|spec| spec.mp).unwrap_or(false);
    let tasks = match growth::list_tasks(&account, mp).await {
        Ok(tasks) => tasks,
        Err(error) => return management_error(502, error),
    };
    let Some(task) = growth::find_task(&tasks, &request.code) else {
        return ok_json(json!({
            "id": account_id,
            "code": request.code,
            "outcome": "skipped",
            "message": "任务不存在（可能已下线）",
            "credit": 0,
            "energy": 0,
        }));
    };
    let report = if request.mode == "claim" {
        // 只领奖：未完成的任务不动（服务端对未完成任务 claim 返回 not completed）
        let accept_status = task.get("accept_status").and_then(Value::as_str).unwrap_or("");
        let progress = task.get("progress").cloned().unwrap_or(Value::Null);
        let current = progress.get("current").and_then(Value::as_i64).unwrap_or(0);
        let target = progress.get("target").and_then(Value::as_i64).unwrap_or(1);
        if accept_status != "completed" && accept_status != "claimed" && current < target {
            growth::TaskReport {
                code: request.code.clone(),
                outcome: TaskOutcome::Skipped,
                message: format!("未完成（{current}/{target}），未点亮不领取"),
                credit: 0,
                energy: 0,
            }
        } else {
            // 小程序限定任务的 claim 必须带 `X-Client-Platform: miniprogram`
            // （缺头返回 task not found，实测）—— mp 判据来自映射表，别写死。
            let (result, credit, energy, message) =
                growth::claim_task(&account, &request.code, mp).await;
            growth::TaskReport {
                code: request.code.clone(),
                outcome: match result {
                    1 => TaskOutcome::Done,
                    2 => TaskOutcome::Already,
                    _ => TaskOutcome::Failed,
                },
                message,
                credit,
                energy,
            }
        }
    } else if request.code == "first_buddy" {
        growth::process_first_buddy(&account, &task).await
    } else {
        growth::process_task(&account, &request.code, &task).await
    };
    ok_json(json!({
        "id": account_id,
        "code": report.code,
        "outcome": report.outcome.as_str(),
        "message": report.message,
        "credit": report.credit,
        "energy": report.energy,
    }))
}

/// `POST /api/accounts/{id}/growth/school` —— 开学季任务 + 可选抽奖。
///
/// `only` 限定任务码；`lottery=true` 时任务之后把抽奖余额抽空（上限 20 次护栏）。
/// school 任务体量小（4 个映射任务 × 每次几秒），一次请求的时长在管理 API 的
/// 60 秒窗口内；若未来任务数增长导致超时，按单任务端点同款拆分。
pub async fn school_run(state: &ServerState, account_id: &str, body: &Bytes) -> Response {
    let account = match growth_account(state, account_id) {
        Ok(account) => account,
        Err(response) => return response,
    };
    let request = parse_request(body);
    let (reports, lottery) =
        school::run_all(&account, &request.only, request.lottery).await;
    let rows: Vec<Value> = reports
        .iter()
        .map(|report| {
            json!({
                "code": report.code,
                "outcome": report.outcome,
                "message": report.message,
            })
        })
        .collect();
    ok_json(json!({
        "id": account_id,
        "results": rows,
        "lottery": lottery.unwrap_or(Value::Null),
    }))
}

/// `POST /api/accounts/{id}/growth/travel` —— 猫猫旅行巡检一次。
///
/// 一趟只推进一步（无猫 → 领养；idle → 派出；arrived → 领奖；traveling → 跳过）。
pub async fn travel_run(state: &ServerState, account_id: &str) -> Response {
    let account = match growth_account(state, account_id) {
        Ok(account) => account,
        Err(response) => return response,
    };
    let report = travel::run_once(&account).await;
    ok_json(json!({
        "id": account_id,
        "outcome": report.outcome,
        "message": report.message,
        "credit": report.credit,
    }))
}
