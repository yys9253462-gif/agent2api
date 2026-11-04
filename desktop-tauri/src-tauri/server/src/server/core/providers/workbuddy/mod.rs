//! WorkBuddy（腾讯编码代理客户端）：**两个地区 = 两家 provider** 的身份定义。
//!
//! **实现已整体搬到提供商插件**（`agent2api-providers`，不随本仓库发布）：
//! 端点与客户端身份、出站请求体归一（角色 / tool_choice / 前缀缓存键 /
//! tool 配对）、token 续期、`/v3/config` 目录刷新、每日活跃保活、成长任务树
//! （任务 / 开学季 / 猫猫旅行）全在那边。网关只留身份：
//!
//! ```text
//!   region.rs   地区 ↔ provider id ↔ ProviderKind 的互查、展示名、缓存槽
//!   账号面      `account_store`（默认 provider id 与存量记录的公开形态）
//!   模型规则    `core::model_rules`（用户配置，与别家同一条链路）
//! ```
//!
//! ── 这一家的历史地位（读旧注释时别误会）─────────────────────
//! 它是本项目的**原始唯一上游**：`ProviderKind`、账号存储、模型目录最初都是
//! 围绕它写的，因此代码里到处能看到「workbuddy 语义」的兜底（默认 provider id、
//! 默认登录态、`user-<uid>` 账号 id 形态）。那些是历史契约，不是别的家可以
//! 照抄的范本 —— 也正因为如此，`DEFAULT_PROVIDER_ID` 仍然指向 `workbuddy`：
//! 存量账号的 `provider` 字段缺省时按它解释。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本模块零 unwrap/expect/panic。

pub mod region;

pub use region::{is_workbuddy_family, Region};
