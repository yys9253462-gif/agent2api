//! ModelScope（魔搭，`modelscope` 国内站 / `modelscope-intl` 国际站）的**身份**。
//!
//! ── 实现已整体搬到提供商插件 ────────────────────────────────
//! 转发（OpenAI 兼容）、模型目录、魔粒余额、每日签到、令牌探活全在插件里。
//! 网关这边只剩两件事：
//!   - 枚举值与 id / 标签（[`crate::server::core::providers::ProviderKind`]）；
//!   - 地区 ↔ provider 的互查（[`region::Region`]）—— 那是账号落盘契约的
//!     一部分（`provider` 字段决定账号归哪家、签到打哪个站点）。
//!
//! ── panic=abort ────────────────────────────────────────────
//! 本文件不碰对话链路，也不做任何 IO。

pub mod region;
pub use region::Region;
