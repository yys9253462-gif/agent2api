//! ModelScope（魔搭）的**地区身份**（网关侧只剩这一件事）。
//!
//! ── 这个模块为什么还在 ──────────────────────────────────────
//! 实现（转发 / 目录 / 魔粒余额 / 签到 / 探活）已经整体搬到提供商插件。网关
//! 不再知道「魔搭的两个站点各是哪个域名」—— 那是上游知识。
//!
//! 但**两个站点 = 两个 provider** 这个身份划分是**落盘契约**：账号记录里的
//! `provider` 字段、优先级队列、界面上两张卡片、签到范围判定，全按
//! `modelscope` / `modelscope-intl` 分开。所以这里只留：
//!   - provider id ↔ 地区的互查（`from_kind` / `from_provider_id` / `kind`）；
//!   - 界面标签与账号 id 前缀。
//!
//! ── 为什么是两个 provider 而不是「一家的一个选项」───────────────
//! 地区做成「账号上的一个字段」的后果与本仓库已修过数次的那类缺陷相同：
//! 地区成了**账号的属性**，界面上混在一起、账号记录也无法按站隔离。拆成两个
//! provider 后各家有独立的账号、清单、启停与映射。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use crate::server::core::providers::{kind_id, ProviderKind};

/// ModelScope 的地区。
///
/// 顺序 = 注册表顺序（国内版在前）：`ALL` 的遍历顺序决定界面上两家的先后。
/// **国内版在前**（用户直觉里「魔搭就是国内那个站」，与 MonkeyCode / ZCode
/// 同一处置）。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Region {
    /// 国内站（provider id `modelscope`）
    #[default]
    Cn,
    /// 国际站（provider id `modelscope-intl`）
    Intl,
}

impl Region {
    /// 全部地区（遍历用）
    pub const ALL: [Region; 2] = [Region::Cn, Region::Intl];

    /// 本地区的 provider
    pub fn kind(self) -> ProviderKind {
        match self {
            Self::Cn => ProviderKind::Modelscope,
            Self::Intl => ProviderKind::ModelscopeIntl,
        }
    }

    /// 本地区的 provider id
    pub fn provider_id(self) -> &'static str {
        kind_id(self.kind())
    }

    /// provider id → 地区；不是 ModelScope 系时 None
    pub fn from_provider_id(provider_id: &str) -> Option<Self> {
        match provider_id {
            "modelscope" => Some(Self::Cn),
            "modelscope-intl" => Some(Self::Intl),
            _ => None,
        }
    }

    /// provider kind → 地区；不是 ModelScope 系时 None
    pub fn from_kind(kind: ProviderKind) -> Option<Self> {
        match kind {
            ProviderKind::Modelscope => Some(Self::Cn),
            ProviderKind::ModelscopeIntl => Some(Self::Intl),
            _ => None,
        }
    }

    /// 展示名（界面上跟在 `ModelScope` 后面的那一段）
    pub fn label(self) -> &'static str {
        match self {
            Self::Cn => "国内版",
            Self::Intl => "国际版",
        }
    }

    /// 本地区账号记录 id 的**前缀**（id 生成用）。
    ///
    /// 两地带前缀的理由与 AutoClaw 国际版那一节相同：两站的身份 id 空间互不相通，
    /// 但**同一个人在两地的 `HavanaId` 完全可能相同**（各自注册，服务端 id 生成
    /// 规则同形就会撞）。撞了的话存储层现有的跨 provider 撞 id 保护会直接报错拒绝，
    /// 「两地账号并存」就变得不可能 —— 而并存正是本功能的意义之一。
    ///
    /// 本家没有存量账号，所以国内版**不必**保持裸 id，两家都带前缀（Cline 同款）。
    pub fn account_id_prefix(self) -> &'static str {
        match self {
            Self::Cn => "modelscope-",
            Self::Intl => "modelscope-intl-",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Region;

    /// 两条身份必须与注册表里的 id 逐字对上 —— 对不上的话 `adapter_for` 会返回
    /// 错误的适配器实例（转发打到另一个站点），而这是编译期看不出来的。
    #[test]
    fn region_ids_match_the_expected_literals() {
        assert_eq!(Region::Cn.provider_id(), "modelscope");
        assert_eq!(Region::Intl.provider_id(), "modelscope-intl");
    }

    /// 互查是双射：id → region → id 必须回到原值
    #[test]
    fn region_lookup_round_trips() {
        for region in Region::ALL {
            assert_eq!(Region::from_provider_id(region.provider_id()), Some(region));
            assert_eq!(Region::from_kind(region.kind()), Some(region));
        }
        assert_eq!(Region::from_provider_id("loomy"), None);
    }

    /// 账号 id 前缀必须**不同** —— 相同的话同一身份在两地的记录会生成同一个 id，
    /// 第二个添加的账号会撞上存储层的跨 provider 保护而加不进来。
    ///
    /// 注意这里**不**断言「互不为前缀」：`modelscope-intl-` 本就以 `modelscope-`
    /// 开头（Cline 的两个池同款）。判「是不是 ModelScope 系」一律走
    /// `Region::from_provider_id`，绝不要用 `starts_with` 前缀判据 —— 那会把国际版
    /// 误算进国内版。
    #[test]
    fn account_id_prefixes_are_distinct() {
        assert_ne!(Region::Cn.account_id_prefix(), Region::Intl.account_id_prefix());
    }
}
