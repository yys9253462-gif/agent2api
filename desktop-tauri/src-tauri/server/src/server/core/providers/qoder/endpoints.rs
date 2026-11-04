//! Qoder 的**地区身份**（中国版 / 国际版是两家 provider）。
//!
//! ── 实现已整体搬到提供商插件 ─────────────────────────────────
//! COSY 签名、请求体编码、SSE 信封拆解、设备授权、续期、额度、签到、风控
//! （UMID 组件）、目录与转发全在 `agent2api-providers` 里（不随本仓库发布）。
//! 网关这边只留**身份**：地区 ↔ provider id ↔ `ProviderKind` 的互查，以及
//! 界面要用的展示字段（`id` / `edition` / `label`）。
//!
//! ── 地区即 provider 身份（2026-10 拆家）────────────────────────
//! 与 AutoClaw / Accio / ZCode 同款：两个地区是**两家 provider**（`qoder` 中国版
//! / `qoder-intl` 国际版），地区 → 身份的互查就在本 impl，别处不要再写
//! `"qoder-intl"` 字面量。中国版保持裸 `qoder`（存量账号 `accounts` 里的落盘
//! 契约，只改展示名）。
//!
//! ── 上游域名为什么也删了 ────────────────────────────────────
//! 两站的网关、openapi、门户域名都是「怎么打上游」的知识，随实现一起搬到
//! 插件。网关手里留着它们只会变成一份没人维护、却看起来权威的副本。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic。

use serde_json::Value;

use crate::server::core::providers::{kind_id, ProviderKind};
use crate::server::errors::GatewayError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    Global,
    Cn,
}

impl Region {
    /// 两个地区（注册表顺序：中国版在前 —— 历史已有的一家，存量账号都归它）。
    pub const ALL: [Region; 2] = [Region::Cn, Region::Global];

    /// 本地区对应哪个 provider kind（地区 → 身份的**唯一**映射）
    pub fn kind(self) -> ProviderKind {
        match self {
            Self::Cn => ProviderKind::Qoder,
            Self::Global => ProviderKind::QoderIntl,
        }
    }

    /// 本地区的 provider id（`"qoder"` / `"qoder-intl"`）
    pub fn provider_id(self) -> &'static str {
        kind_id(self.kind())
    }

    /// provider id → 地区（`qoder` 系之外的 id 返回 None）
    pub fn from_provider_id(provider_id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|region| region.provider_id() == provider_id)
    }

    /// 这个 kind 是不是 Qoder 系（两家都算）—— 判据只在这里写一份
    pub fn from_kind(kind: ProviderKind) -> Option<Self> {
        Self::ALL.into_iter().find(|region| region.kind() == kind)
    }

    /// 解析地区标识（`global` / `intl` / `cn`，空串按国际版）。
    ///
    /// 只为**读旧数据**服务：存量记录 / 导入文件里的 `mode` 字段。新写入的
    /// 账号由 provider id 决定地区。
    pub fn parse(value: &str) -> Result<Self, GatewayError> {
        match value.trim() {
            "" | "global" | "intl" => Ok(Self::Global),
            "cn" => Ok(Self::Cn),
            _ => Err(GatewayError::with_status(
                400,
                "Qoder 地区必须为 global（国际版）或 cn（中国版）",
            )),
        }
    }

    /// 从记录的 `mode` / `edition` 字段读地区（缺省国际版，与旧口径一致）。
    pub fn from_payload(payload: &Value) -> Result<Self, GatewayError> {
        let mode = payload.get("mode").or_else(|| payload.get("edition"));
        match mode {
            None | Some(Value::Null) => Ok(Self::Global),
            Some(Value::String(value)) => Self::parse(value),
            _ => Err(GatewayError::with_status(400, "Qoder 地区必须是字符串")),
        }
    }

    /// 地区的稳定短名（进记录 / 日志 / 前端回显）。
    pub fn id(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Cn => "cn",
        }
    }

    /// 版本标识（公开形态的 `edition` 字段，前端据此分组与显示）。
    pub fn edition(self) -> &'static str {
        match self {
            Self::Global => "intl",
            Self::Cn => "cn",
        }
    }

    /// 展示名（`editionLabel` 与日志用）。
    pub fn label(self) -> &'static str {
        match self {
            Self::Global => "国际版",
            Self::Cn => "中国版",
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
    }

    /// 旧数据的 `mode` / `edition` 解析口径固定：缺失按国际版，
    /// `cn` 是中国版，其余非法值报 400。
    #[test]
    fn legacy_region_values_parse_stably() {
        assert_eq!(Region::Global, Region::parse("").unwrap_or(Region::Cn));
        assert_eq!(Region::Global, Region::parse("global").unwrap_or(Region::Cn));
        assert_eq!(Region::Global, Region::parse("intl").unwrap_or(Region::Cn));
        assert_eq!(Region::Cn, Region::parse("cn").unwrap_or(Region::Global));
        assert!(Region::parse("mars").is_err());
        // 记录的 edition 字段（拆家前写的是 `intl` / `cn`）
        assert_eq!(
            Region::Global,
            Region::from_payload(&serde_json::json!({ "edition": "intl" }))
                .unwrap_or(Region::Cn)
        );
        assert_eq!(
            Region::Cn,
            Region::from_payload(&serde_json::json!({ "mode": "cn" })).unwrap_or(Region::Global)
        );
    }
}
