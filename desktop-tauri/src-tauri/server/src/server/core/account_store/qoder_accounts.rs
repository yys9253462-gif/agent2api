//! Qoder 账号以 provider（地区身份）+ userId 识别；凭证与其它提供商共享存储键名。
//!
//! ── 拆家（2026-10）──────────────────────────────────────────
//! 两个地区是**两家 provider**（`qoder` 中国版 / `qoder-intl` 国际版），地区由
//! provider id 推导（`Region::from_provider_id`），不再从账号的 `mode` 字段猜。
//! 记录里仍写 `mode`（凭证落盘的既有字段，公开形态的 `edition` 也由它派生），
//! 但**归属判定只认 provider id** —— 与 AutoClaw / Accio / ZCode 同一口径。
//!
//! ── 实现搬走之后（2026-10 插件化）───────────────────────────
//! 设备授权登录、PAT 换证、续期、目录、额度这些**上游协议知识**已随实现搬到
//! 提供商插件，本文件保留账号层的网关职责：
//!
//! 1. **归属判定**（`qoder_account_record`）：续期分派靠它认出「这条记录属于
//!    本家」，否则会落到 workbuddy 兜底链路、给出与本家无关的错误。
//! 2. **存量记录的公开形态**（`to_qoder_public_account`）：升级上来的老账号没有
//!    `pluginSecret`，走不到插件投影；不在这里补形状就会被渲染成 WorkBuddy 的
//!    空壳（uid 空、tokenTail 空），账号明明能用界面却像坏了。
//! 3. **地区迁移与身份匹配**：缺少地区字段不能作为国际版的证据；同一 userId
//!    在两个地区是两条账号，重新登录复用原主键以保留请求历史。
//!
//! 新落的账号由插件写入（凭证在 `pluginSecret` 里），公开形态直接走插件投影。

use std::sync::MutexGuard;

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::server::core::providers::qoder::endpoints::Region;

use super::sql;
use super::state::StoredAccount;
use super::store::AccountStore;

fn explicit_region(fields: &Map<String, Value>) -> Option<Region> {
    let value = ["mode", "edition"].into_iter().find_map(|key| {
        fields.get(key).filter(|value| {
            !value.is_null() && !matches!(value, Value::String(text) if text.trim().is_empty())
        })
    })?;
    let text = value.as_str()?.trim();
    Region::from_provider_id(text).or_else(|| Region::parse(text).ok())
}

fn plugin_region(record: &StoredAccount) -> Option<Region> {
    // 公开快照可能在错分后的续期中被重写，恢复时优先取凭证原有的地区。
    record.get("pluginSecret").and_then(Value::as_object).and_then(explicit_region)
        .or_else(|| record.get("pluginPublic").and_then(Value::as_object).and_then(explicit_region))
}

pub(crate) fn migrated_region(record: &StoredAccount) -> Option<Region> {
    let region = Region::from_provider_id(&record.provider())?;
    if !record.has_plugin_secret() {
        return explicit_region(record.fields()).or(Some(region));
    }
    // 插件账号的 provider 是权威；只对有明确国内版证据的错分国际版记录归位。
    if region == Region::Global && plugin_region(record) == Some(Region::Cn) {
        Some(Region::Cn)
    } else {
        Some(region)
    }
}

pub(super) fn migrate_accounts(accounts: &mut [StoredAccount]) -> usize {
    let mut changed = 0;
    for record in accounts {
        let Some(region) = migrated_region(record) else { continue; };
        if record.provider() != region.provider_id() {
            record.set_provider(region.provider_id());
            changed += 1;
        }
    }
    changed
}

fn user_id_of(fields: &Map<String, Value>) -> Option<String> {
    ["userId", "user_id", "uid"].into_iter().find_map(|key| match fields.get(key) {
        Some(Value::String(text)) if !text.trim().is_empty() => Some(text.trim().to_string()),
        Some(Value::Number(number)) => Some(number.to_string()),
        _ => None,
    })
}

fn record_user_id(record: &StoredAccount) -> Option<String> {
    record.get("pluginSecret").and_then(Value::as_object).and_then(user_id_of)
        .or_else(|| user_id_of(record.fields()))
}

impl AccountStore {
    pub(super) fn qoder_account_for_upsert(
        &self,
        guard: &MutexGuard<'_, ()>,
        region: Region,
        identity: &str,
        secret: &Value,
    ) -> Result<(String, Option<StoredAccount>), String> {
        let mut records = self.with_conn(guard, |conn| {
            let ids: Vec<String> = sql::queue_rows(conn)?.into_iter()
                .filter(|row| Region::from_provider_id(&row.provider).is_some())
                .map(|row| row.id).collect();
            ids.into_iter().map(|id| sql::load_by_id(conn, &id))
                .collect::<rusqlite::Result<Vec<_>>>()
        }).map_err(|error| error.message)?;
        let user_id = secret.as_object().and_then(user_id_of);
        let existing = records.drain(..).flatten()
            .filter(|record| migrated_region(record) == Some(region))
            .filter(|record| match user_id.as_ref() {
                Some(user_id) => record_user_id(record).as_ref() == Some(user_id),
                None => record.id() == identity,
            })
            .min_by_key(|record| (record.added_at(), record.order_key(), record.id().to_string()));
        if let Some(record) = existing {
            return Ok((record.id().to_string(), Some(record)));
        }
        let id = format!("qoder-{}-{:x}", region.id(),
            Sha256::digest(user_id.as_deref().unwrap_or(identity).as_bytes()));
        if self.with_conn(guard, |conn| sql::load_by_id(conn, &id))
            .map_err(|error| error.message)?.is_some() {
            return Err("Qoder 账号 ID 已被其它账号占用，请先核对账号记录".to_string());
        }
        Ok((id, None))
    }

    /// 某一地区的账号记录：非空 id 按 id 直查（核对归属），空 id 取该地区
    /// **队首可用**账号（启用 + 有令牌，按全局优先级序）—— 与
    /// `autoclaw_account_record` 同一款形状（拆家后「队首」必须限定在本地区）。
    pub fn qoder_account_record(&self, region: Region, account_id: &str) -> Option<Value> {
        let guard = self.guard();
        let provider = region.provider_id();
        if !account_id.is_empty() {
            // 按 id 直查一行后确认归属（同 id 只会有一条，provider 判定是保险）
            let record = self.record_by_id(&guard, account_id)?;
            return (record.provider() == provider).then(|| record.to_value());
        }
        self.records_for_provider(&guard, provider)
            .into_iter()
            .filter(|record| record.enabled() && record.has_token())
            .min_by_key(|record| record.order_key())
            .map(|record| record.to_value())
    }

    pub fn to_qoder_public_account(&self, record: &StoredAccount) -> Value {
        // 地区先看 provider 身份（拆家后的权威来源），payload 里的 `mode` 只是
        // 旧数据的兼容读取 —— 记录缺 `mode` 时 `from_payload` 会默认 Global，
        // 那个默认对中国版记录是错的，不能当首选。
        let region = Region::from_provider_id(&record.provider())
            .or_else(|| Region::from_payload(&record.to_value()).ok());
        let available = record.has_token() && region.is_some();
        // 能不能自动续期只看记录里有没有 refreshToken —— 网关不再解析这家的
        // 凭据形状（`Credentials` 随实现搬到插件了）。判据与插件侧
        // `credentials.can_refresh()` 等价：非空即算。
        let can_refresh = !record.refresh_token().trim().is_empty();
        let mut public = Map::new();
        for key in ["id", "provider", "name", "userId", "email", "nickname", "mode", "source", "tokenTail", "expiresAt"] {
            public.insert(key.to_string(), record.get(key).cloned().unwrap_or(Value::Null));
        }
        public.insert("edition".to_string(), region.map(|value| Value::String(value.edition().to_string())).unwrap_or(Value::Null));
        public.insert("editionLabel".to_string(), region.map(|value| Value::String(value.label().to_string())).unwrap_or(Value::Null));
        public.insert("hasRefreshToken".to_string(), Value::Bool(can_refresh));
        public.insert("priority".to_string(), Value::from(record.priority()));
        public.insert("enabled".to_string(), Value::Bool(record.enabled()));
        public.insert("addedAt".to_string(), Value::from(record.added_at()));
        public.insert("updatedAt".to_string(), Value::from(record.updated_at()));
        public.insert("proxy".to_string(), crate::server::core::proxies::describe_account_proxy(Some(&record.proxy())));
        public.insert("rateLimits".to_string(), record.get("rateLimits").cloned().unwrap_or_else(|| json!({})));
        public.insert("desktop".to_string(), Value::Bool(false));
        public.insert("available".to_string(), Value::Bool(available));
        // `chatSupported` / `maxConcurrent` **刻意不在这里写**：它们是跨家的统一
        // 事实，由 `store_view::public_account` 在分派点注入（前者按适配器的
        // `supports_chat()`，后者按 `store_util::max_concurrent_public_for`）。
        Value::Object(public)
    }
}
