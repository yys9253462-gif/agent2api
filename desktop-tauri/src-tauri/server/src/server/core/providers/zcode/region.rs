//! ZCode 的**地区身份**（国内版 / 国际版是两家 provider）。
//!
//! ── 实现已整体搬到提供商插件 ─────────────────────────────────
//! 登录端点、编码套餐推理基址、Anthropic 通道、监控平面、凭证换取业务域、
//! 环境变量覆盖、设备标识生成 —— 全是「怎么打上游」的知识，随实现一起搬到
//! `agent2api-providers`（不随本仓库发布）。
//!
//! 网关这边只留**身份**：地区 ↔ provider id ↔ `ProviderKind` 的互查，以及
//! 界面要用的展示字段（`label` / `account_id_prefix`）。
//!
//! ── 为什么是两个 provider 而不是「一家的一个选项」──────────
//! 与 Cline 的两个额度池、AutoClaw / Accio / Qoder 的两个地区同一思路：做成
//! 「一个 provider 上的 `region` 字段」会让地区变成**账号的属性**，界面上混在
//! 一起、「哪个账号走哪个站点」看不出来，账号记录也无法按地区隔离。
//! 按两个 provider 建模之后各自有独立的账号、清单、启停与映射。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use crate::server::core::providers::{kind_id, ProviderKind};

/// ZCode 的地区。
///
/// 顺序 = 注册表顺序（国内版在前）：`ALL` 的遍历顺序决定界面上两家的先后
/// （国内网络环境下它是更常被添加的那一个，与 AutoClaw 的排序理由一致）。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Region {
    /// 国内版（智谱开放平台；provider id 是 `zcode`）
    #[default]
    Cn,
    /// 国际版（Z.AI；provider id 是 `zcode-intl`）
    Intl,
}

impl Region {
    /// 两个地区（注册表顺序：国内版在前）
    pub const ALL: [Region; 2] = [Region::Cn, Region::Intl];

    /// 本地区对应哪个 provider kind（地区 → 身份的**唯一**映射）
    pub fn kind(self) -> ProviderKind {
        match self {
            Self::Cn => ProviderKind::Zcode,
            Self::Intl => ProviderKind::ZcodeIntl,
        }
    }

    /// 本地区的 provider id（`"zcode"` / `"zcode-intl"`）
    pub fn provider_id(self) -> &'static str {
        kind_id(self.kind())
    }

    /// provider id → 地区（`zcode` 系之外的 id 返回 None）
    pub fn from_provider_id(provider_id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|region| region.provider_id() == provider_id)
    }

    /// 这个 kind 是不是 ZCode 系（两家都算）—— 判据只在这里写一份
    pub fn from_kind(kind: ProviderKind) -> Option<Self> {
        Self::ALL.into_iter().find(|region| region.kind() == kind)
    }

    /// 展示名（界面上跟在 `ZCode` 后面的那一段）
    pub fn label(self) -> &'static str {
        match self {
            Self::Cn => "国内版",
            Self::Intl => "国际版",
        }
    }

    /// 本地区账号记录 id 的**前缀**（id 生成用）。
    ///
    /// 两家都带前缀：带前缀能让两地的记录天然不相交 —— 同一个人在国内 /
    /// 国际两套系统里的 userId 完全可能相同（与 AutoClaw 国际版同一处境），
    /// 撞了的话存储层的撞 id 保护会拒绝写入，而「两地账号并存」正是两个
    /// provider 建模的意义之一。
    ///
    /// 插件侧生成账号 id 时读的是同一个前缀（那是一条落盘契约）。
    pub fn account_id_prefix(self) -> &'static str {
        match self {
            Self::Cn => "zcode-user-",
            Self::Intl => "zcode-intl-user-",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 地区 ↔ provider id ↔ kind 必须双射（落盘契约）。
    #[test]
    fn region_and_provider_id_are_bijective() {
        for region in Region::ALL {
            assert_eq!(Some(region), Region::from_provider_id(region.provider_id()));
            assert_eq!(Some(region), Region::from_kind(region.kind()));
        }
        assert_eq!(None, Region::from_provider_id("workbuddy"));
        assert_eq!(None, Region::from_provider_id("zcode-intl-2"));
    }

    /// 两个地区的账号 id 前缀必须不同（否则两地的账号会撞 id）。
    #[test]
    fn account_id_prefixes_never_collide() {
        let cn = Region::Cn.account_id_prefix();
        let intl = Region::Intl.account_id_prefix();
        assert_ne!(cn, intl);
        // 前缀不能互相包含：任一方向的前缀关系都会让「按前缀判地区」出错
        assert!(!intl.starts_with(cn) && !cn.starts_with(intl));
    }
}
