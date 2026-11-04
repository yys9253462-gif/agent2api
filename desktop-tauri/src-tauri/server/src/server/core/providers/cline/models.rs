//! Cline（官方 `api.cline.bot`）的**身份**：两个额度池 = 两家 provider。
//!
//! ── 实现已整体搬到提供商插件 ─────────────────────────────────
//! 凭证格式（`workos:` 前缀）、设备授权登录、续期、目录拉取、伪装头、余额、
//! 转发全在 `agent2api-providers` 里（不随本仓库发布）。网关这边只留三件
//! **必须留在身份/配置侧**的事：
//!
//! ```text
//!   1. 池 ↔ provider id ↔ ProviderKind 的互查（`Pool`）—— 它是落盘契约，
//!      账号记录的 `provider` 字段就是这两个 id；
//!   2. 上游 id 的通道前缀（`cline-free/` / `cline-pass/`）—— 它是**计费通道
//!      选择器**，模型规则（alias → target）与路由都要用它判「这条映射属于
//!      哪个池」，而模型规则是用户配置、住在网关；
//!   3. `friendly_alias` 的剥前缀规则 —— 默认映射种子（`model_rules::cline`）
//!      要用它算「这条模型该有的短名」。
//! ```
//!
//! ── 两个额度池为什么是两家 provider ─────────────────────────
//! `cline-free` 与 `cline-pass` 在上游是**同一个账号上的两个额度池**，但本项目
//! 把它们当**两个提供商**接入：各自有独立的账号、清单、启停规则与映射。
//!
//! 早先的建模是「一家的一个选项」（账号记录带 `pool` 字段，池过滤按「账号库里
//! 有哪个池」决定广告哪一批）。那套做法有三个说不通的地方：
//!   - 界面上一家 Cline 里混着两个池的模型，**还随账号变化**；
//!   - 模型的启停规则键是「(provider, 模型 id)」，两个池共用一个 provider，
//!     关掉某家的一个模型会影响另一个池的同名模型；
//!   - 同名模型（`deepseek-v4.1-flash` 两池都有）的短名归属要靠「账号里有哪个
//!     池」来排序决定，规则随账号漂移。
//!
//! 拆成两家后池过滤变成**身份的固有属性**：插件 `list` 只列这个池的模型，
//! 与账号状态无关。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use crate::server::core::providers::{kind_from_id, kind_id, ProviderKind};

/// 免费池前缀（上游 id 的通道标记之一）
pub const FREE_PREFIX: &str = "cline-free/";

/// 订阅池前缀（上游 id 的通道标记之一）
pub const PASS_PREFIX: &str = "cline-pass/";

/// 云池前缀（**已知但不收录**：实测 403）
pub const CLOUD_PREFIX: &str = "cline-cloud/";

/// 额度池。**一个池 = 一家提供商**，所以这个枚举同时是「这家是谁」的定义点
/// —— 池与 provider id / `ProviderKind` 的互查都在这里，别处不要再写
/// `"cline-free"` 这类字面量。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pool {
    /// 免费额度池（`cline-free/...` 与免费组的无前缀条目）
    Free,
    /// ClinePass 订阅池（`cline-pass/...`）
    Pass,
}

impl Pool {
    /// 两个池（注册表顺序：Free 在前）。
    pub const ALL: [Pool; 2] = [Pool::Free, Pool::Pass];

    /// 解析 `pool` / 提供商 id 的各种写法（容忍 `cline-free`、`free`、`Free`）。
    ///
    /// ── 缺省为什么是 `pass`（迁移期的历史口径）──────────────────
    /// 这个函数现在只为**读旧数据**服务：存量账号记录带着 `pool: "free"` /
    /// `"pass"` 字段（拆分前写入的），迁移时按它决定账号归哪一家。缺省 `pass`
    /// 沿用旧口径 —— 旧实现里字段缺失或值非法都回落 `pass`，迁移必须给出
    /// **逐字相同**的结果，否则用户升一次级就换了池。
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "free" | "cline-free" => Self::Free,
            _ => Self::Pass,
        }
    }

    /// 这个池现在是哪家 provider（**唯一**池 → 身份映射）
    pub fn kind(self) -> ProviderKind {
        match self {
            Self::Free => ProviderKind::ClineFree,
            Self::Pass => ProviderKind::ClinePass,
        }
    }

    /// 这个池的 provider id（`"cline-free"` / `"cline-pass"`）
    pub fn provider_id(self) -> &'static str {
        kind_id(self.kind())
    }

    /// provider id → 池（别家的 id 返回 None）
    pub fn from_provider_id(provider_id: &str) -> Option<Self> {
        kind_from_id(provider_id).and_then(|kind| match kind {
            ProviderKind::ClineFree => Some(Self::Free),
            ProviderKind::ClinePass => Some(Self::Pass),
            _ => None,
        })
    }

    /// 上游 id 的**通道前缀**（`cline-free/` / `cline-pass/`）。
    ///
    /// 这个前缀必须原样发上游（是计费通道选择器），也是「某条映射属于哪个池」
    /// 的判据（见 `catalog::wire_target_for_provider`）。
    pub fn target_prefix(self) -> &'static str {
        match self {
            Self::Free => FREE_PREFIX,
            Self::Pass => PASS_PREFIX,
        }
    }
}

/// 上游 id → 池（**按 id 前缀猜的兜底判据**）。
///
/// ── 它只在三处用，别扩散 ────────────────────────────────────
/// 远程目录的归池由**插件按响应里的分组**决定（免费组里有不带 `cline-free/`
/// 前缀的裸 id，按前缀猜会把它们错判进订阅池）。网关这边用它只有三处：
///   - **拆分迁移**的判据（`model_rules::cline::migrate_cline_split`）：存量账号
///     的 `pool` 字段当初就是按这个口径写的，迁移必须给出逐字相同的结果；
///   - 模型规则的旧条目里 target 不带前缀时判归属；
///   - 测试与诊断。
///
/// 无前缀的 id 归 `Pass`：前缀都没带时无从判断通道，归 pass 是**保守**选择。
pub fn pool_of(model_id: &str) -> Pool {
    if model_id.starts_with(FREE_PREFIX) {
        return Pool::Free;
    }
    Pool::Pass
}

/// 上游 id → 对下游的**友好名**（这条模型该有的去前缀写法）；没有可剥的返回 None。
///
/// ── 按池判，而不是只看 id ────────────────────────────────────
/// 免费池里混着**不带通道前缀**的条目（`z-ai/glm-5.3-flash`、
/// `poolside/laguna-s-2.1:free`）：它们的通道由上游分组给出，id 上只有承载方的
/// 厂商前缀。这类条目同样该有一个能直接敲的短名字，于是：
///   - 带通道前缀（`cline-free/` / `cline-pass/` / `cline-cloud/`）→ 剥通道前缀；
///   - **免费池**里不带通道前缀、但带厂商前缀（`z-ai/…`）→ 剥厂商前缀；
///   - 其余（订阅池的无前缀条目、`recommended` 组的 `openai/gpt-6-astra` 这类）
///     → `None`。那里厂商前缀是**模型身份**（`openai/gpt-6-astra` 与
///     `anthropic/claude-opus-5` 是不同模型），剥掉会造出上游不存在的名字。
///
/// ── 撞名（明确接受）─────────────────────────────────────────
/// 剥出来的短名可能与别家的原生 id 撞（`glm-5.3-flash` 同时是 CatPaw 的原生
/// id）。撞名在本项目的路由语义下是**正常**的：同一 alias 允许多家各一条映射、
/// 一起进候选链做主备（见 `model_rules` 模块头），不遮蔽任何东西。
pub fn friendly_alias(pool: Pool, model_id: &str) -> Option<&str> {
    let id = model_id.trim();
    if id.is_empty() {
        return None;
    }
    // 通道前缀优先：`cline-free/deepseek-v4.1-flash` → `deepseek-v4.1-flash`
    for prefix in [FREE_PREFIX, PASS_PREFIX, CLOUD_PREFIX] {
        if let Some(rest) = id.strip_prefix(prefix) {
            return Some(rest);
        }
    }
    // 免费池的裸厂商 id：`z-ai/glm-5.3-flash` → `glm-5.3-flash`。
    // 没有 `/` 的 id（上游本来就给的友好名）无需映射。
    if pool == Pool::Free {
        let (_, rest) = id.split_once('/')?;
        // 再剥变体后缀 `:free`（`poolside/laguna-s-2.1:free` → `laguna-s-2.1`）——
        // Cline 自己的展示名归一化也是这么做的（去掉 `:free` 与 ` (free)` 后缀）。
        // 留着后缀会得到一个客户端难敲的名字。
        let rest = rest.strip_suffix(":free").unwrap_or(rest);
        return (!rest.is_empty()).then_some(rest);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 池 ↔ provider id ↔ kind 必须双射（落盘契约）。
    #[test]
    fn pool_and_provider_id_are_bijective() {
        for pool in Pool::ALL {
            assert_eq!(Some(pool), Pool::from_provider_id(pool.provider_id()));
            assert_eq!(
                Some(pool.kind()),
                kind_from_id(pool.provider_id()),
                "kind 与 provider id 必须互查一致"
            );
        }
        assert_eq!(None, Pool::from_provider_id("cline"));
        assert_eq!(None, Pool::from_provider_id("workbuddy"));
    }

    /// 旧数据里的 `pool` 字段（`free` / `pass` / 缺失 / 非法）解析口径固定：
    /// 缺失与非法一律回落 `pass`，迁移必须给出逐字相同的结果。
    #[test]
    fn legacy_pool_values_parse_stably() {
        assert_eq!(Pool::Free, Pool::parse("free"));
        assert_eq!(Pool::Free, Pool::parse("cline-free"));
        assert_eq!(Pool::Pass, Pool::parse("pass"));
        assert_eq!(Pool::Pass, Pool::parse("cline-pass"));
        assert_eq!(Pool::Pass, Pool::parse(""));
        assert_eq!(Pool::Pass, Pool::parse("nonsense"));
    }

    /// 短名规则：带通道前缀的剥前缀；免费池的裸厂商 id 剥厂商前缀与 `:free`；
    /// 订阅池的裸 id 与 recommended 的厂商 id 不剥（那是模型身份）。
    #[test]
    fn friendly_alias_strips_channels_not_identities() {
        assert_eq!(
            Some("deepseek-v4.1-flash"),
            friendly_alias(Pool::Free, "cline-free/deepseek-v4.1-flash")
        );
        assert_eq!(
            Some("glm-5.3"),
            friendly_alias(Pool::Pass, "cline-pass/glm-5.3")
        );
        assert_eq!(
            Some("glm-5.3-flash"),
            friendly_alias(Pool::Free, "z-ai/glm-5.3-flash")
        );
        assert_eq!(
            Some("laguna-s-2.1"),
            friendly_alias(Pool::Free, "poolside/laguna-s-2.1:free")
        );
        // 订阅池的厂商 id 是模型身份，不剥
        assert_eq!(None, friendly_alias(Pool::Pass, "openai/gpt-6-astra"));
        // 上游本来就给的友好名（无前缀无斜杠）无需映射
        assert_eq!(None, friendly_alias(Pool::Free, "glm-5.3-flash"));
    }

    /// 前缀判池：只有 `cline-free/` 判免费池，其余（含裸 id）归订阅池。
    #[test]
    fn pool_of_only_recognizes_the_free_channel_prefix() {
        assert_eq!(Pool::Free, pool_of("cline-free/deepseek-v4.1-flash"));
        assert_eq!(Pool::Pass, pool_of("cline-pass/glm-5.3"));
        assert_eq!(Pool::Pass, pool_of("z-ai/glm-5.3-flash"));
        assert_eq!(Pool::Pass, pool_of("glm-5.3-flash"));
    }
}
