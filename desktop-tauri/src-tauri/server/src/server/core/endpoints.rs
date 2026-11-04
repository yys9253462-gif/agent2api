//! 版本（edition）标识：`cn` / `intl`。
//!
//! ── 这里原来有什么 ──────────────────────────────────────────
//! 整套上游端点表（`auth` / `llm` / `billing` / `activity` / `config` /
//! `cloudAgent` / `sidecar` 七张）、客户端与 CLI 版本号、UA 模板、鉴权域名
//! 白名单、上游业务码表、`Context`（baseUrl / prefix / platform 的拼装）——
//! 那些是「怎么伪装成官方客户端打上游」的知识，按项目的风险隔离要求已整体
//! 搬到提供商插件（`agent2api-providers`），本仓库不再持有。
//!
//! ── 现在只剩什么 ────────────────────────────────────────────
//! **账号记录里会出现的那两个字符串**：`edition` 的值域（`cn` / `intl`）与
//! 它的展示名。存量账号记录里的 `edition` 字段就是这两个值，别名归一也必须
//! 继续认（`international` / `global` / `ai` …），否则老记录升级后会被当成
//! 未知版本、落进错误的地区分组。
//!
//! 端点 / 前缀 / platform / 客户端版本**不在这里**：网关按记录里已有的值
//! 原样透传，记录里没有就是空串 —— 它不需要知道该填什么。

use std::path::PathBuf;

/// 默认版本 id（存量账号的归属；也是「记录里没写 edition」时的兜底）
pub const DEFAULT_EDITION: &str = "cn";

/// 一个版本的**标识**（落盘值 + 展示名）。
///
/// 刻意只有两个字段：再加任何一项都会变成「网关知道某个上游该怎么打」。
#[derive(Clone, Copy, Debug)]
pub struct EditionInfo {
    /// 落盘值（`cn` / `intl`）—— 存量账号记录的 `edition` 字段就是它
    pub id: &'static str,
    /// 展示名（界面上跟在提供商名后面的那一段）
    pub label: &'static str,
}

/// 国内版
const EDITION_CN: EditionInfo = EditionInfo {
    id: "cn",
    label: "国内版",
};

/// 国际版
const EDITION_INTL: EditionInfo = EditionInfo {
    id: "intl",
    label: "国际版",
};

/// 全部版本，顺序与界面上两家的先后一致（cn → intl）
pub const EDITIONS: &[EditionInfo] = &[EDITION_CN, EDITION_INTL];

/// 宽松解析版本 id：接受 `cn` / `intl` 及若干常见写法，未知值回落国内版。
///
/// 别名表是**兼容性契约**（存量记录、导入文件、旧配置里都出现过这些写法），
/// 不是上游知识 —— 别删。
pub fn resolve_edition(value: Option<&str>) -> &'static EditionInfo {
    let id = value.unwrap_or("").trim().to_lowercase();
    if id.is_empty() || id == "cn" {
        return &EDITION_CN;
    }
    if id == "intl" {
        return &EDITION_INTL;
    }
    if matches!(
        id.as_str(),
        "international" | "global" | "ai" | "workbuddy-ai" | "workbuddyai" | "en"
    ) {
        return &EDITION_INTL;
    }
    if matches!(
        id.as_str(),
        "china" | "mainland" | "workbuddy" | "zh" | "cn-intl"
    ) {
        return &EDITION_CN;
    }
    &EDITION_CN
}

/// 旧版单账号登录态文件（仅用于首次启动迁移到账号列表，见 account_store）
pub fn legacy_auth_file() -> PathBuf {
    crate::server::config::config_dir().join("auth.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 版本 id 的归一必须稳定：存量记录里的 `edition` 是 `cn` / `intl`，
    /// 未知值（含缺失）回落国内版 —— 那是历史数据的归属。
    #[test]
    fn edition_ids_normalize_stably() {
        assert_eq!("cn", resolve_edition(Some("cn")).id);
        assert_eq!("intl", resolve_edition(Some("intl")).id);
        assert_eq!("cn", resolve_edition(None).id);
        assert_eq!("cn", resolve_edition(Some("mars")).id);
        // 别名（兼容旧记录与旧配置）
        assert_eq!("intl", resolve_edition(Some("International")).id);
        assert_eq!("intl", resolve_edition(Some("workbuddy-ai")).id);
        assert_eq!("cn", resolve_edition(Some("workbuddy")).id);
    }
}
