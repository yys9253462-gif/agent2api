//! AutoClaw 的**地区身份**：两个 provider id 与展示名。
//!
//! ── 这里为什么只剩这么点 ────────────────────────────────────
//! AutoClaw 的实现（转发、凭证、刷新、目录、余额、签到、活动、**登录**）已整体
//! 搬到提供商插件。网关侧只保留「这是一家的哪个地区」——它是**落盘契约与界面
//! 分组的依据**，不是上游知识：
//!
//! ```text
//!   autoclaw        国内版（id 不变：存量 accounts.json 里就是它）
//!   autoclaw-intl   国际版
//! ```
//!
//! 域名 / 签名密钥 / 客户端版本 / 回调端口 / 登录码表全部在插件里 ——
//! 主仓库不再持有「怎么伪装成官方客户端」的任何知识（这是拆家的目的）。
//!
//! ── 国内版的 provider id 为什么**不改名** ────────────────────
//! 注册表里 `autoclaw` 这个 id 已经在用户的 accounts.json 里了（`provider` 字段
//! 是落盘契约）。改成 `autoclaw-cn` 会让全部存量账号在升级后变成「未知
//! provider」——`kind_from_id` 返回 None，账号从列表里静默消失。
//! 所以**只改展示名**，id 保持 `autoclaw` 不变；国际版取 `autoclaw-intl`。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use crate::server::core::providers::{kind_id, ProviderKind};

/// AutoClaw 的地区。
///
/// 顺序 = 注册表顺序（国内版在前）：`ALL` 的遍历顺序决定界面上两家的先后。
/// **国内版在前**是因为它是历史已有的那一家（存量账号都归它）。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Region {
    /// 国内版（历史已有的那一家，provider id 仍是 `autoclaw`）
    #[default]
    Cn,
    /// 国际版（provider id 是 `autoclaw-intl`）
    Intl,
}

impl Region {
    /// 两个地区（注册表顺序：国内版在前）
    pub const ALL: [Region; 2] = [Region::Cn, Region::Intl];

    /// 本地区对应哪个 provider kind（地区 → 身份的**唯一**映射）
    pub fn kind(self) -> ProviderKind {
        match self {
            Self::Cn => ProviderKind::AutoClaw,
            Self::Intl => ProviderKind::AutoClawIntl,
        }
    }

    /// 本地区的 provider id（`"autoclaw"` / `"autoclaw-intl"`）
    pub fn provider_id(self) -> &'static str {
        kind_id(self.kind())
    }

    /// provider id → 地区（`autoclaw` 系之外的 id 返回 None）
    pub fn from_provider_id(provider_id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|region| region.provider_id() == provider_id)
    }

    /// 这个 kind 是不是 AutoClaw 系（两家都算）—— 判据只在这里写一份
    pub fn from_kind(kind: ProviderKind) -> Option<Self> {
        Self::ALL.into_iter().find(|region| region.kind() == kind)
    }

    /// 展示名（界面上跟在 `AutoClaw` 后面的那一段）
    pub fn label(self) -> &'static str {
        match self {
            Self::Cn => "国内版",
            Self::Intl => "国际版",
        }
    }
}
