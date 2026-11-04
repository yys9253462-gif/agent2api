//! 插件页的运行期资源：设计令牌、组件库产物、提供商图标。
//!
//! ```text
//! GET /plugins/runtime/theme.css        组件库样式与宿主窗口参数
//! GET /plugins/runtime/islands/ui.js    组件库 + React（跨文档出口）
//! GET /plugins/runtime/islands/ui.css   Tailwind 产物（组件库样式）
//! GET /plugins/runtime/providers/<id>.png  提供商图标
//! ```
//!
//! ── 这些资源解决什么问题 ────────────────────────────────────
//! 插件页面（签到中心这类）在 iframe 里，是**独立文档**：读不到宿主的
//! `ui/css/tokens.css`、拿不到宿主已加载的 React / 组件库，也读不到
//! `ui/assets/providers/` 里的图标。于是它只能自绘一套控件、手抄一份色板、
//! 用首字母代替品牌图标 —— 三样都会与宿主漂移，表现成「迁移过来的页面长得
//! 不一样」。
//!
//! 把这三类资源从网关出给 iframe，插件页就能用**与宿主完全相同的那一份**：
//!   · `theme.css` → 色板 / 圆角 / 阴影 / 滚动条，一个值都不差；
//!   · `islands/ui.js` + `islands/ui.css` → 组件库的同一份实现与样式
//!     （`window.wbUiKit` 暴露 React 与组件，见 ui-islands/src/uikit.tsx）；
//!   · `providers/*.png` → 品牌图标，与宿主账号页 / 模型页用的是同一张图。
//!
//! ── 为什么内嵌进二进制而不是读磁盘 ──────────────────────────
//! 桌面形态的网关**不知道** `ui/` 目录在哪（界面由 Tauri 壳从资源里出，
//! `ServerState::ui_dir` 只有 headless 才设置，见 `http.rs` 的静态托管段）。
//! 让网关去猜路径会在桌面形态下静默 404。这几份文件用 `include_str!` /
//! `include_bytes!` 编进二进制，两种形态行为一致，也不引入「文件被外部删了」
//! 这种失败模式。体积代价：`ui.js` 约 1.3 MB（未压缩），exe 相应变大。
//!
//! ── 不挂 protected ──────────────────────────────────────────
//! 与 `/plugins/{pack}/ui/*` 同一理由：iframe 的请求带不上面板令牌头，而这些
//! 文件不含任何数据 —— 它们就是 `ui/` 目录里那几份公开静态资源（`ui/index.html`
//! 同样不鉴权）。真正的数据请求仍走 `/api/*`（全部 protected）。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use axum::body::Body;
use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::Response;

/// 宿主窗口参数（设计令牌由 ISLANDS_CSS 提供）。
const TOKENS: &str = include_str!("../../../../../ui/css/tokens.css");

/// 组件库 + React 的 iife 产物（`ui-islands` 构建时落盘）。
const ISLANDS_JS: &str = include_str!("../../../../../ui/islands/ui.js");

/// Tailwind 产物（组件库样式）。
const ISLANDS_CSS: &str = include_str!("../../../../../ui/islands/ui.css");

/// 保留资源路径兼容；共享布局组件的样式已经包含在 ISLANDS_CSS 中。
const LAYOUT_CSS: &str = include_str!("../../../../../ui/css/layout.css");

/// 保留业务排版与旧资源路径，基础控件样式由组件库提供。
const COMPONENTS_CSS: &str = include_str!("../../../../../ui/css/components.css");

/// 提供商图标：`(provider id, PNG 字节)`。
///
/// ── 为什么用一张静态表而不是读目录 ──────────────────────────
/// `include_bytes!` 必须在编译期知道每个路径，没法遍历目录。这份表因此是
/// 「图标文件清单」的第二处声明 —— 加一张新图标时要在这里补一行。
/// 漏补的表现是插件页显示首字母而不是图标（不是崩溃），代价可控。
///
/// 只列**插件家**用得到的那些（签到中心只画有签到能力的家）。宿主自己用的
/// 其余图标不必在这里出现：它们走 `ui/assets/providers/` 的常规路径。
const PROVIDER_ICONS: &[(&str, &[u8])] = &[
    ("workbuddy", include_bytes!("../../../../../ui/assets/providers/workbuddy.png")),
    ("raccoon", include_bytes!("../../../../../ui/assets/providers/raccoon.png")),
    ("catpaw", include_bytes!("../../../../../ui/assets/providers/catpaw.png")),
    ("autoclaw", include_bytes!("../../../../../ui/assets/providers/autoclaw.png")),
    ("qoder", include_bytes!("../../../../../ui/assets/providers/qoder.png")),
    ("cline", include_bytes!("../../../../../ui/assets/providers/cline.png")),
    ("accio", include_bytes!("../../../../../ui/assets/providers/accio.png")),
    ("zcode", include_bytes!("../../../../../ui/assets/providers/zcode.png")),
    ("codearts", include_bytes!("../../../../../ui/assets/providers/codearts.png")),
    ("trae", include_bytes!("../../../../../ui/assets/providers/trae.png")),
    ("loomy", include_bytes!("../../../../../ui/assets/providers/loomy.png")),
    ("kuku", include_bytes!("../../../../../ui/assets/providers/kuku.png")),
    ("monkeycode", include_bytes!("../../../../../ui/assets/providers/monkeycode.png")),
    ("commandcode", include_bytes!("../../../../../ui/assets/providers/commandcode.png")),
    ("antigravity", include_bytes!("../../../../../ui/assets/providers/antigravity.png")),
    ("officeace", include_bytes!("../../../../../ui/assets/providers/officeace.png")),
    ("orcaterm", include_bytes!("../../../../../ui/assets/providers/orcaterm.png")),
    ("chatglm", include_bytes!("../../../../../ui/assets/providers/chatglm.png")),
    ("modelscope", include_bytes!("../../../../../ui/assets/providers/modelscope.png")),
    ("atomcode", include_bytes!("../../../../../ui/assets/providers/atomcode.png")),
    ("lobsterai", include_bytes!("../../../../../ui/assets/providers/lobsterai.png")),
];

const MINIMAX_CODE_SVG: &[u8] = include_bytes!("../../../../../ui/assets/providers/minimax-code.svg");

/// 响应构造的小工具：省掉每个分支里重复的 builder 链。
fn reply(status: StatusCode, mime: &'static str, body: Vec<u8>) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, mime)
        // 与插件页其它资源同一口径：网关升级后立刻拿到新产物，
        // 不会出现「页面用了新组件、js 还是旧的」这种中间态
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(body))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap_or_else(|_| Response::new(Body::empty()))
        })
}

/// GET /plugins/runtime/{*rest}
///
/// 一个入口分派四种资源：路径匹配是显式的（`match`），不做前缀猜测 ——
/// 猜错的后果是「请求了个不存在的文件却拿到了别的东西」，比 404 更难查。
pub async fn serve(Path(rest): Path<String>) -> Response {
    // 归一化：去掉空段与 `.`，`..` 一律拒绝（这里只有固定几个文件，
    // 不需要「向上走」的语义）
    let normalized: Vec<&str> = rest
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect();
    if normalized.iter().any(|segment| *segment == ".." || segment.contains('\\')) {
        return reply(StatusCode::NOT_FOUND, "text/plain; charset=utf-8", b"not found".to_vec());
    }
    let path = normalized.join("/");
    match path.as_str() {
        "theme.css" => reply(
            StatusCode::OK,
            "text/css; charset=utf-8",
            format!("{ISLANDS_CSS}\n{TOKENS}").into_bytes(),
        ),
        "islands/ui.js" => reply(
            StatusCode::OK,
            "text/javascript; charset=utf-8",
            ISLANDS_JS.as_bytes().to_vec(),
        ),
        "islands/ui.css" => reply(
            StatusCode::OK,
            "text/css; charset=utf-8",
            ISLANDS_CSS.as_bytes().to_vec(),
        ),
        "layout.css" => reply(
            StatusCode::OK,
            "text/css; charset=utf-8",
            LAYOUT_CSS.as_bytes().to_vec(),
        ),
        "components.css" => reply(
            StatusCode::OK,
            "text/css; charset=utf-8",
            COMPONENTS_CSS.as_bytes().to_vec(),
        ),
        "tooltip.css" => reply(
            StatusCode::OK,
            "text/css; charset=utf-8",
            ISLANDS_CSS.as_bytes().to_vec(),
        ),
        "providers/minimax-code.svg" | "providers/minimax-code-global.svg" => reply(
            StatusCode::OK,
            "image/svg+xml",
            MINIMAX_CODE_SVG.to_vec(),
        ),
        other => {
            // 提供商图标：`providers/<id>.png`
            let Some(name) = other.strip_prefix("providers/") else {
                return reply(StatusCode::NOT_FOUND, "text/plain; charset=utf-8", b"not found".to_vec());
            };
            let Some(id) = name.strip_suffix(".png") else {
                return reply(StatusCode::NOT_FOUND, "text/plain; charset=utf-8", b"not found".to_vec());
            };
            match PROVIDER_ICONS.iter().find(|(key, _)| *key == id) {
                Some((_, bytes)) => reply(StatusCode::OK, "image/png", bytes.to_vec()),
                None => reply(StatusCode::NOT_FOUND, "text/plain; charset=utf-8", b"not found".to_vec()),
            }
        }
    }
}
