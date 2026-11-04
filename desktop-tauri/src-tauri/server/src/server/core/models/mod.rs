//! 模型目录的**出口形状**工具（`/v1/models` 的条目与清单信封）。
//!
//! ── 这里为什么只剩 shape ────────────────────────────────────
//! 本模块原先还带着 WorkBuddy 的**目录本体**：内置模型清单、`/v3/config`
//! 远程刷新、白名单过滤、按地区分槽的持久化缓存。那些都是「怎么打上游拉清单」
//! 的知识，随 WorkBuddy 的实现整体搬到提供商插件了（不随本仓库发布）。
//!
//! 留下来的是 [`shape`]：它把**任何一家**的模型条目翻成下游认的 OpenAI 形态
//! （`id` / `object` / `owned_by` / 能力位 / 上下文窗口），并组装 `/v1/models`
//! 的响应信封。这部分是网关的出口契约，与上游是谁无关 —— 插件把自己的清单
//! 交给网关后，走的正是这条翻译链。
//!
//! ── 消费方 ────────────────────────────────────────────────
//!   · `core::providers::catalog`（聚合目录：把各家清单拼成一份）；
//!   · `api::pipeline`（按名字匹配模型时用的同一套文本化规则）；
//!   · `core::providers::catalog::view`（`/v1/models` 的条目的最终形态）。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本模块零 unwrap/expect/panic。

pub mod shape;

// 对外再导出：聚合目录（`core::providers::catalog`）与排障方从 `core::models`
// 取这些名字，保持与拆分前同一条导入路径。
// `shape_value_text` 是 `shape::value_text` 的别名导出：聚合目录要按 `name`
// 匹配模型，需要与 `shape` 内部同一套 JS 文本化。
pub use shape::{list_item, list_response_from, model_id, suggest_from, value_text as shape_value_text};
