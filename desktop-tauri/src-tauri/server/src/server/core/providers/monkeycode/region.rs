//! MonkeyCode 的**地区身份**（国内版 / 国际版）—— 只保留落盘与展示需要的那部分。
//!
//! ── 这份文件为什么只剩身份 ──────────────────────────────────
//! 本家的转发、目录、账号探活全部搬到了**提供商插件**
//! （`agent2api-providers` 的 `monkeycode/`），不随本仓库发布 —— 那是风险隔离
//! 的一部分。站点域名、cookie 名、任务流协议这些上游知识都随插件走了。
//!
//! 网关这边仍然需要「地区」这个概念，因为它是**落盘契约**：
//!   - 两个 provider id（`monkeycode` / `monkeycode-intl`）各自独立成组；
//!   - 账号 id 带地区前缀，同一个人在两地注册不会撞 id；
//!   - 界面上两家的分组名要显示「国内版 / 国际版」。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use crate::server::core::providers::{kind_id, ProviderKind};

/// MonkeyCode 的地区。
///
/// 顺序 = 注册表顺序（国内版在前）：`ALL` 的遍历顺序决定管理面分派与界面上
/// 两家的先后。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Region {
    /// 国内版（provider id `monkeycode`）
    #[default]
    Cn,
    /// 国际版（provider id `monkeycode-intl`）
    Intl,
}

impl Region {
    /// 两个地区（注册表顺序：国内版在前）
    pub const ALL: [Region; 2] = [Region::Cn, Region::Intl];

    /// 本地区对应哪个 provider kind（地区 → 身份的**唯一**映射）
    pub fn kind(self) -> ProviderKind {
        match self {
            Self::Cn => ProviderKind::MonkeyCode,
            Self::Intl => ProviderKind::MonkeyCodeIntl,
        }
    }

    /// 本地区的 provider id（`"monkeycode"` / `"monkeycode-intl"`）
    pub fn provider_id(self) -> &'static str {
        kind_id(self.kind())
    }

    /// provider id → 地区（`monkeycode` 系之外的 id 返回 None）
    pub fn from_provider_id(provider_id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|region| region.provider_id() == provider_id)
    }

    /// 这个 kind 是不是 MonkeyCode 系（两家都算）—— 判据只在这里写一份
    pub fn from_kind(kind: ProviderKind) -> Option<Self> {
        Self::ALL.into_iter().find(|region| region.kind() == kind)
    }

    /// 地区短标识（`"cn"` / `"intl"`，用于日志）
    pub fn id(self) -> &'static str {
        match self {
            Self::Cn => "cn",
            Self::Intl => "intl",
        }
    }

    /// 展示名后缀（界面上跟在 `MonkeyCode` 后面的那一段）
    pub fn label(self) -> &'static str {
        match self {
            Self::Cn => "国内版",
            Self::Intl => "国际版",
        }
    }

    /// 本地区账号记录 id 的**前缀**（id 生成用）。
    ///
    /// 两地的 provider id 已经把归属分开，但账号 id 在**整份账号集合里唯一**：
    /// 同一个人在两地各自注册、服务端 id 生成规则一旦同形就会撞。带上前缀之后
    /// 两地天然不相交，同一个人的两个地区账号可以并存。
    pub fn account_id_prefix(self) -> &'static str {
        match self {
            Self::Cn => "monkeycode-",
            Self::Intl => "monkeycode-intl-",
        }
    }
}
