//! Accio 的**地区身份**与纯本地常量（网关侧只剩这一件事）。
//!
//! ── 这个模块为什么还在 ──────────────────────────────────────
//! 实现（转发 / 目录 / 额度 / 登录 / 续期）已经整体搬到提供商插件。网关不再
//! 知道「Accio 的上游长什么样、怎么换令牌」—— 那些是反向代理的实现细节。
//!
//! 但**两个地区 = 两个 provider** 这个身份划分是**落盘契约**：账号记录里的
//! `provider` 字段、优先级队列、界面上的两张卡片、签到范围判定，全都按
//! `accio` / `accio-cn` 分开。所以这里只留：
//!   - provider id ↔ 地区的互查（`from_kind` / `from_provider_id` / `kind`）；
//!   - 落进账号记录的 `edition` 值（`intl` / `cn`）与界面标签。
//!
//! 上游域名、端点路径、`appKey`、设备指纹这些**一条都不留** —— 它们在插件里。
//!
//! ── 为什么地区是一等公民而不是「一家上的一个字段」────────────────
//! 与 `autoclaw::region` / Cline 两池同一条思路：地区做成账号上的字段，
//! 界面与选路就分不清「哪条账号走哪个站点」，而落错家会让转发稳定打错站点。

use crate::server::core::providers::{kind_id, ProviderKind};

/// Accio 的两个地区。**两个地区 = 两个 provider**。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Region {
    /// 国际版
    Global,
    /// 国内版
    Cn,
}

impl Region {
    /// 全部地区（缓存数组、遍历用）
    pub const ALL: [Region; 2] = [Region::Global, Region::Cn];

    /// 本地区的 provider
    pub const fn kind(self) -> ProviderKind {
        match self {
            Self::Global => ProviderKind::Accio,
            Self::Cn => ProviderKind::AccioCn,
        }
    }

    /// 本地区的 provider id（`"accio"` / `"accio-cn"`）
    pub fn provider_id(self) -> &'static str {
        kind_id(self.kind())
    }

    /// provider id → 地区；不是 Accio 系时 None
    pub fn from_provider_id(provider_id: &str) -> Option<Self> {
        match provider_id {
            "accio" => Some(Self::Global),
            "accio-cn" => Some(Self::Cn),
            _ => None,
        }
    }

    /// provider kind → 地区；不是 Accio 系时 None
    pub fn from_kind(kind: ProviderKind) -> Option<Self> {
        match kind {
            ProviderKind::Accio => Some(Self::Global),
            ProviderKind::AccioCn => Some(Self::Cn),
            _ => None,
        }
    }

    /// 落进账号记录的 `edition` 值（与其它家同一套取值：`intl` / `cn`）
    pub const fn edition(self) -> &'static str {
        match self {
            Self::Global => "intl",
            Self::Cn => "cn",
        }
    }

    /// 界面与日志里的名字
    pub const fn label(self) -> &'static str {
        match self {
            Self::Global => "国际版",
            Self::Cn => "国内版",
        }
    }
}
