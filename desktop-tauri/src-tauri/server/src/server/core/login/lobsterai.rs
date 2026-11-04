//! LobsterAI 的网页登录收尾（**loopback 回调路由专用**）。
//!
//! ── 这条链属于哪一型 ────────────────────────────────────────
//! 与 Accio / AutoClaw 同族：**浏览器把用户带回本机 HTTP 端口**
//! （`GET /auth/callback?code=…&state=…`）。授权地址与换码都在**插件**里
//! （`agent2api-providers` 的 `lobsterai/oauth.rs`），网关这边只做三件事：
//! 校验这次回调属于哪个登录任务 → 把整条回调交给插件换码 → 写回任务状态。
//!
//! ── 与 `submit_login_callback` 的关系 ───────────────────────
//! 那个函数服务「壳侧把回调 URL 交回来」的通用入口（`POST
//! /api/session/login/callback`），插件接管的家在那里走同一条
//! `exchange_login_code`。本文件是**第二条**入口：浏览器直接导航到
//! `/auth/callback`，路由拿不到 `ServerState` 之外的东西，所以收尾单独写。
//! 两条路最终都落到插件的 `login.callback` RPC。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use serde_json::{Value, json};

use crate::server::core::providers::{kind_id, ProviderKind};
use crate::server::errors::GatewayError;
use crate::server::logging;

use super::{LoginService, finish_task_error};

impl LoginService {
    /// 浏览器回调：校验归属 → 用授权码换凭证 → 落账号 → 标记任务完成。
    ///
    /// ── 校验口径（与 accio 同一套，逐条都有理由）───────────────
    /// 回调落在本机 HTTP 端口上，任何本机进程都能伪造一次 GET，因此：
    ///   1. `state` 必须对应一个**进行中且属于 LobsterAI** 的登录任务；
    ///   2. 插件侧那一轮 pending 由它自己取走即失效（`login.callback` 里
    ///      `take_pending`）——「授权码只能兑一次」的闸门在插件进程里；
    ///   3. 任务已完成时直接返回成功：用户刷新页面 / 浏览器预取会重复回调，
    ///      第二次必然拿到「授权码已失效」，把一次成功变成一次失败。
    pub async fn finish_lobsterai_login(
        &self,
        code: &str,
        state: &str,
    ) -> Result<String, GatewayError> {
        let code = code.trim();
        let state = state.trim();
        if code.is_empty() {
            return Err(GatewayError::with_status(400, "回调没有携带授权码"));
        }
        if state.is_empty() {
            return Err(GatewayError::with_status(
                400,
                "回调没有携带 state，无法确认这次登录归属",
            ));
        }
        let Some(handle) = self.tasks.get(state) else {
            return Err(GatewayError::with_status(
                404,
                "这次登录已取消或已过期，请重新发起",
            ));
        };
        let provider = kind_id(ProviderKind::LobsterAi);
        if handle.snapshot().provider != provider {
            return Err(GatewayError::with_status(400, "这次登录不属于 LobsterAI"));
        }
        if handle.snapshot().done {
            // 幂等：重复回调不是错误（见校验口径 3）
            return Ok(String::new());
        }
        let Some(adapter) = crate::server::core::providers::adapter::adapter_by_id(provider)
        else {
            return Err(GatewayError::with_status(
                400,
                "LobsterAI 的提供商插件没有安装，无法完成登录",
            ));
        };
        match adapter.exchange_login_code(&self.store, code, state).await {
            Ok(account_id) => {
                handle.update(|task| {
                    task.done = true;
                    task.session = Some(json!({
                        "accountUid": account_id,
                        "nickname": Value::Null,
                        "provider": provider,
                    }));
                    task.finished_at = Some(logging::now_ms());
                });
                logging::log("[Login]", "✅ LobsterAI 网页登录完成，账号已加入列表");
                Ok(account_id)
            }
            Err(error) => {
                finish_task_error(&handle, &error.message);
                logging::log(
                    "[Login]",
                    &format!("❌ LobsterAI 网页登录换取凭证失败: {}", error.message),
                );
                Err(error)
            }
        }
    }
}
