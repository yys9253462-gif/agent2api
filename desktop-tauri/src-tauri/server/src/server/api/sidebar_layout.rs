//! 侧边栏布局的读写接口：`GET/PUT /api/sidebar-layout`。
//!
//! ── 为什么要有这一对端点 ────────────────────────────────────
//! 布局从前只存浏览器 localStorage，而桌面壳（`tauri://localhost`）与网关
//! 托管的网页端（`http://127.0.0.1:<port>`）是**两个 origin**，localStorage
//! 天然隔离 —— 在 WebUI 里隐藏的菜单，客户端照旧显示。落库 + 一对端点之后
//! 两端读同一份，这是「设置在哪改都一样」的唯一实现方式。
//!
//! ── 契约 ────────────────────────────────────────────────────
//! ```text
//! GET  /api/sidebar-layout  → { layout: <布局对象> | null }
//! PUT  /api/sidebar-layout  → 收 { layout: <布局对象> | null }，回同一份
//! ```
//!   · `layout: null` = 库里没存过 / 用户恢复默认 → 前端回落 `defaultLayout()`；
//!   · 其余形状一律 400（顶层必须是对象或 null）。
//!
//! ── 为什么后端只判「顶层是不是对象」────────────────────────
//! 布局的合法形状由前端 `ui/sidebar-layout.js` 的 `MENUS` 表定义，而**插件页
//! 的菜单键是运行期才有的**。后端若照抄一份键白名单，前端加菜单或装插件之后
//! 这份校验就会把合法数据判成非法 —— 校验只该有一处，且该在拥有那张表的一侧。
//! 详见 `core::sidebar_layout` 的模块头。
//!
//! ── 读不到时为什么不报错 ────────────────────────────────────
//! 「库里没有布局」是**首启动的正常状态**，不是异常：返回 `layout: null`
//! 让前端走默认布局。若这里回 404 / 500，前端就要为「端点不存在」和
//! 「真没数据」写两套兜底 —— 而两者的正确动作完全一样。

use axum::body::Bytes;
use axum::extract::State;
use axum::response::Response;
use serde_json::{json, Value};

use crate::server::core::sidebar_layout;
use crate::server::errors;
use crate::server::http::{ok_json, parse_body};
use crate::server::ServerState;

/// GET /api/sidebar-layout
pub async fn get_layout(State(_state): State<ServerState>) -> Response {
    ok_json(json!({ "layout": sidebar_layout::load() }))
}

/// PUT /api/sidebar-layout —— body `{layout: {...} | null}`
///
/// 缺 `layout` 键与显式 `null` 同义（都表示「恢复默认」）：前端两种写法都
/// 可能发出（省略字段 / 传 null），区分它们不会带来任何行为差异。
pub async fn put_layout(State(_state): State<ServerState>, body: Bytes) -> Response {
    let payload = match parse_body(&body) {
        Ok(value) => value,
        Err(error) => return errors::management_error(400, error.message),
    };
    let Some(object) = payload.as_object() else {
        return errors::management_error(400, "请求体必须是 JSON 对象");
    };
    // 缺字段 = 恢复默认（见上面注释）
    let layout: Option<&Value> = match object.get("layout") {
        None | Some(Value::Null) => None,
        Some(value @ Value::Object(_)) => Some(value),
        Some(_) => {
            return errors::management_error(400, "layout 必须是 JSON 对象或 null");
        }
    };
    if let Err(message) = sidebar_layout::save(layout) {
        return errors::management_error(400, message);
    }
    ok_json(json!({ "layout": sidebar_layout::load() }))
}
