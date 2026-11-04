//! WorkBuddy 的**地区身份**（国内版 / 国际版是两家 provider）。
//!
//! ── 实现已整体搬到提供商插件 ─────────────────────────────────
//! 端点 / 客户端版本 / 伪装头、出站请求体归一（角色 / tool_choice /
//! 前缀缓存键 / tool 配对）、token 续期、`/v3/config` 目录刷新、每日活跃
//! 保活、成长任务树（任务 / 开学季 / 猫猫旅行）、环境变量旁路 —— 全是
//! 「怎么伪装成官方客户端打上游」的知识，按项目的风险隔离要求不留在本仓库。
//!
//! 网关这边只留**身份**：地区 ↔ provider id ↔ `ProviderKind` 的互查、展示名、
//! 模型清单的缓存槽键，以及版本 id 的归一（`cn` / `intl` 是**落盘契约**：
//! 存量账号记录里的 `edition` 字段就是这两个值）。
//!
//! ── 为什么两个地区是两家 provider ───────────────────────────
//! 与 Cline 的两个额度池、AutoClaw / Accio / ZCode 的两个地区同一思路：把地区
//! 做成「账号上的 `edition` 字段」会让目录只有一份（两个地区互相覆盖，issue
//! #74）、模型命名空间只有一套（同名模型无法区分，issue #89）、转发选路也不含
//! 地区（请求可能落到另一个地区的账号上）。按两家建模之后，目录、缓存槽、
//! 刷新排期、模型规则、别名、账号队列、报表全都按 provider id 自动分开。
//!
//! ── provider id 与落盘契约 ──────────────────────────────────
//! 国内版**必须**保持 `"workbuddy"`：它是存量账号里的落盘契约，改名会让那些
//! 账号升级后变成「未知 provider」而静默消失。它同时是 `DEFAULT_PROVIDER_ID`
//! （历史数据都来自单上游时代）。
//! 国际版用新 id `"workbuddy-intl"`，存量国际版账号由启动迁移原地归位。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use crate::server::core::endpoints::{resolve_edition, EditionInfo};
use crate::server::core::providers::{kind_id, ProviderKind};

/// WorkBuddy 的地区。
///
/// 顺序 = 注册表顺序（国内版在前）：`ALL` 的遍历顺序决定界面上两家的先后。
/// 国内版在前是因为它是存量账号的归属。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Region {
    /// 国内版（provider id 是 `workbuddy`）
    #[default]
    Cn,
    /// 国际版（provider id 是 `workbuddy-intl`）
    Intl,
}

impl Region {
    /// 两个地区（注册表顺序：国内版在前）
    pub const ALL: [Region; 2] = [Region::Cn, Region::Intl];

    /// 本地区对应哪个 provider kind（地区 → 身份的**唯一**映射）
    pub fn kind(self) -> ProviderKind {
        match self {
            Self::Cn => ProviderKind::WorkBuddy,
            Self::Intl => ProviderKind::WorkBuddyIntl,
        }
    }

    /// 本地区的 provider id（`"workbuddy"` / `"workbuddy-intl"`）
    pub fn provider_id(self) -> &'static str {
        kind_id(self.kind())
    }

    /// provider id → 地区（`workbuddy` 系之外的 id 返回 None）
    pub fn from_provider_id(provider_id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|region| region.provider_id() == provider_id)
    }

    /// 这个 kind 是不是 workbuddy 系（两家都算）—— 判据只在这里写一份
    pub fn from_kind(kind: ProviderKind) -> Option<Self> {
        Self::ALL.into_iter().find(|region| region.kind() == kind)
    }

    /// 展示名（界面上跟在 `WorkBuddy` 后面的那一段）
    pub fn label(self) -> &'static str {
        match self {
            Self::Cn => "国内版",
            Self::Intl => "国际版",
        }
    }

    /// 本地区的版本标识（落盘值 + 展示名）。
    ///
    /// 只有这两个字段 —— 端点 / 前缀 / platform / 客户端身份全在插件里，
    /// 网关不再持有（见 `core::endpoints` 的模块头）。
    pub fn edition(self) -> &'static EditionInfo {
        resolve_edition(Some(self.id()))
    }

    /// 本地区的版本 id（`"cn"` / `"intl"`；落盘契约，别改）
    pub fn id(self) -> &'static str {
        match self {
            Self::Cn => "cn",
            Self::Intl => "intl",
        }
    }

    /// 本地区模型清单的**持久化缓存槽**（`catalog_cache` 的键）。
    ///
    /// 一个地区一槽是拆分的要害：拆家前两个地区共用一槽，谁刷谁覆盖
    /// （issue #74）。
    pub fn catalog_scope(self) -> &'static str {
        match self {
            Self::Cn => crate::server::core::providers::catalog_cache::SCOPE_WORKBUDDY,
            Self::Intl => crate::server::core::providers::catalog_cache::SCOPE_WORKBUDDY_INTL,
        }
    }

    /// 把版本 id 归一成地区（接受 `cn` / `intl` 及常见别名，未知值回落国内版）
    /// —— 别名表在 `resolve_edition` 里，别处不要另写一份。
    ///
    /// 判据取归一后的 **id 字符串**而不是「返回的引用是不是同一个」：
    /// `resolve_edition` 返回的 `&'static` 来自 `const` 提升，两次调用是否
    /// 落在同一个地址**不是**语言保证（指针比较会时对时错）。
    pub fn from_edition_id(value: Option<&str>) -> Self {
        let id = resolve_edition(value).id;
        Self::ALL
            .into_iter()
            .find(|region| region.id() == id)
            .unwrap_or(Self::Cn)
    }
}

/// 这个 provider id 是不是 **workbuddy 系**（国内版 / 国际版都算）。
///
/// 判据只在这里写一份。**什么时候该用它**：两家共用同一套账号 schema
/// （`uid` 身份 + `edition` / `endpoint` / `prefixPath` / `platform` 与
/// accessToken/refreshToken 凭证形态），因此凡是「按 schema 分派」的地方
/// （账号导入导出的字段归一、身份提取、凭证形状判定）都必须把它当成一家 ——
/// 写成 `provider == kind_id(ProviderKind::WorkBuddy)` 会让国际版掉进
/// 「别家」那一支（导入时字段被清空、导出的账号认不出身份）。
///
/// **什么时候不该用它**：任何需要区分地区的地方（目录、缓存槽、账号队列、
/// 模型规则、报表）—— 那些地方用 `Region` 本身或 provider id。
pub fn is_workbuddy_family(provider_id: &str) -> bool {
    Region::from_provider_id(provider_id).is_some()
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
        assert_eq!(None, Region::from_provider_id("zcode"));
    }

    /// 版本 id 的归一必须稳定：存量账号记录里的 `edition` 是 `cn` / `intl`，
    /// 未知值（含缺失）回落国内版 —— 那是历史数据的归属。
    #[test]
    fn edition_ids_normalize_stably() {
        assert_eq!(Region::Cn, Region::from_edition_id(Some("cn")));
        assert_eq!(Region::Intl, Region::from_edition_id(Some("intl")));
        assert_eq!(Region::Cn, Region::from_edition_id(None));
        assert_eq!(Region::Cn, Region::from_edition_id(Some("mars")));
    }
}
