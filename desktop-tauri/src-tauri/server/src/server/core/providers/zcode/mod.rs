//! ZCode（智谱 / Z.AI 的编码代理客户端）的**地区身份**：国内版 / 国际版两家 provider。
//!
//! ── 实现已整体搬到提供商插件 ─────────────────────────────────
//! CLI 轮询登录、编码套餐凭证换取、活动套餐通道（Anthropic 信封 + 官方三段
//! 系统提示词）、限时套餐领取、激活事件、余额与监控窗口、人机验证令牌池、
//! 端点映射、GLM-5.x 思考档位契约、模型静态表与适配器全在
//! `agent2api-providers` 里（不随本仓库发布）。
//!
//! 网关这边只留三样东西：
//!
//! ```text
//!   region.rs        地区 ↔ provider id ↔ ProviderKind 的互查、展示字段
//!   账号面           `account_store::zcode_accounts`（存量记录的公开形态投影）
//!   通道选择         `zcodePlan` 字段的取值域（纯标签与白名单，不含上游知识）
//! ```
//!
//! ── 为什么「两条通道」这个开关留在网关 ───────────────────────
//! `zcodePlan` 是**账号记录上的一个键**，界面的下拉、账号页的显示、换通道后的
//! 限额清理都按它走。它只声明「用哪条额度」，不含任何「怎么打上游」的内容 ——
//! 两条通道各自的 URL、协议、鉴权头全在插件里。所以这里留的是**值域**
//! （`coding-plan` / `start-plan`）与**展示名**，与别家的枚举同性质。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本模块零 unwrap/expect/panic。

pub mod region;

/// 账号记录上的「用哪条通道」字段名（`accounts.json` 的键；界面读同一个名字）。
///
/// 为什么带 `zcode` 前缀而不是光叫 `plan`：账号记录是**各家共用**的一张形状
/// （`StoredAccount` 的裸字段表），`plan` 这个词在别家（Cline 的套餐、
/// Trae 的 planType）都有各自含义，裸名迟早撞车；带前缀之后前端、存储、
/// 网关三处读的是同一个字符串，不存在映射表。
pub const PLAN_FIELD: &str = "zcodePlan";

/// 编码套餐通道（默认值）：用登录拿到的访问令牌走本家的编码套餐额度。
pub const PLAN_CODING: &str = "coding-plan";

/// 活动套餐通道（套餐池）：编码套餐之外的另一种额度，用套餐 JWT 鉴权。
///
/// 这条通道打哪个端点、说什么协议、怎么装配身份提示词，全在插件里 —— 网关
/// 只知道「这个值表示另一条额度」。
pub const PLAN_START: &str = "start-plan";

/// 归一化界面/接口给的通道取值：只认上面两个常量，其余（含空值）返回 None。
///
/// 取值口径与插件侧的套餐 `planId` 一样**不猜**：认不出的值一律拒绝，
/// 而不是静默回落到默认通道 —— 那会让用户以为切过去了，实际还在用旧通道。
pub fn normalize_plan(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        PLAN_CODING => Some(PLAN_CODING),
        PLAN_START => Some(PLAN_START),
        _ => None,
    }
}

/// 账号（**公开形态**）用哪条通道。
///
/// 缺失/认不出 = [`PLAN_CODING`]：这是存量账号的默认，也是「添加账号后直接能用」
/// 的那条路（默认行为因此与本次改造之前逐字节相同）。
pub fn plan_of(account: &serde_json::Value) -> &'static str {
    account
        .get(PLAN_FIELD)
        .and_then(serde_json::Value::as_str)
        .and_then(normalize_plan)
        .unwrap_or(PLAN_CODING)
}

/// 通道的展示名：账号页的下拉与保存后的变更提示共用这一处措辞
/// （两处各写一遍的话，用户会在弹窗里看到「编码套餐」、在日志里看到
/// 「Coding Plan」，对不上是同一条设置）。
///
/// Start Plan 这条叫「**套餐池**」而不是早期的「活动套餐」：那个池子里除了
/// 官方限时活动发放的额度，还有**新注册用户的免费体验额度** —— 叫「活动套餐」
/// 会让恰恰最需要这条通道的用户以为与己无关。
pub fn plan_label(plan: &str) -> &'static str {
    match plan {
        PLAN_START => "套餐池（Start Plan）",
        _ => "编码套餐（Coding Plan）",
    }
}

// ── 「插件读网关用户配置」的两个约定键（`settings.get` 的 `key`）──────────
// ZCode 的官方三段提示词（正文 + 说明）随实现搬到插件了，但「装不装」
// 「改过哪几段」仍是**网关的用户配置**（设置页那一栏）。插件通过这两个键读
// 同一份配置，网关侧的回答在 `plugin_host::bootstrap` 里（它认得这两个名字，
// 不会去查 `plugin.<provider>.<key>`）。

/// 「网关自带提示词」的开关（缺失 = 装，与网关侧的默认值一致）。
pub const SETTING_PROMPT_ENABLED: &str = "gatewayPrompt";

/// 「网关自带提示词」的正文覆盖（`{identity, stable, dynamic}`，只含改过的段）。
pub const SETTING_PROMPT_TEXT: &str = "gatewayPromptText";
