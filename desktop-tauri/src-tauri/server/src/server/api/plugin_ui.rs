//! 插件 UI 资源的托管：`GET /plugins/{pack}/ui/{*rest}`。
//!
//! ── 为什么由网关托管而不是客户端自己读包 ──────────────────────
//! 插件页面是包里的静态资源，而**只有网关知道包装在哪**（`{config_dir}/plugins/
//! <id>/<version>/ui/`）。手机端连的是网关，它连那台机器的文件系统都看不到 ——
//! 所以页面必须由网关出。
//!
//! ── 为什么这条路由**不挂 protected** ────────────────────────
//! 它是页面本身（HTML + JS + CSS），要能被 iframe 直接加载；而 iframe 里的
//! 请求带不上面板令牌头。页面拿到令牌靠的是宿主用 `postMessage` 传进去
//! （见客户端的 `plugin-frame` 桥接）。真正的数据请求仍走 `/api/*`，那些
//! 全部挂 protected —— 页面本身不含任何数据，只是代码。
//!
//! 这是与「静态 `ui/` 目录」同一档的暴露面：`ui/index.html` 同样不鉴权。
//!
//! ── 安全 ────────────────────────────────────────────────────
//! 路径解析与边界检查全在 `plugin_host::resolve_ui_file`：只服务**已启用**包的
//! `ui/` 目录，`..` 与绝对路径一律拒绝。这里只负责读文件 + content-type。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use axum::body::Body;
use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::Response;

use crate::server::core::plugin_host;
use crate::server::errors;

/// GET /plugins/{pack}/ui/{*rest}
pub async fn serve(Path((pack, rest)): Path<(String, String)>) -> Response {
    let Some(path) = plugin_host::resolve_ui_file(&pack, &rest) else {
        return errors::not_found_response("GET", "plugin-ui");
    };
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(_) => return errors::not_found_response("GET", "plugin-ui"),
    };
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_of(&path))
        // 与静态服务同一口径：插件升级后浏览器立刻拿到新页面，
        // 不会出现「页面引用了旧 js」的中间态
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(bytes));
    response.unwrap_or_else(|_| errors::not_found_response("GET", "plugin-ui"))
}

/// 扩展名 → content-type。插件页面是一组前端资源，常见类型够用。
fn mime_of(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "webp" => "image/webp",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "map" => "application/json",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}
