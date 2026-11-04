//! 智谱清言（`chatglm.cn`）的**存量记录**：访客标记 + 令牌位置迁移。
//!
//! 实现已整体搬到提供商插件（`agent2api-providers`，不随本仓库发布）：访客换证 /
//! 粘贴令牌 / 导入本机登录态、续期、目录、转发、余额、签到、任务中心全在那边。
//! 网关这边只剩两件**必须留在数据侧**的事：
//!
//! ```text
//!   1. `guest_of_record`      —— 公开形态的 `isGuest` 与签到范围判定读的同一份
//!                               事实。存量记录（没有 `pluginSecret`）没有插件
//!                               替它们回答这个问题，而「访客不在签到范围」是
//!                               网关侧的集合划分（`billing::checkin`）。
//!   2. `migrate_nested_tokens` —— 首版把令牌写进嵌套 `auth`，启动时抬到顶层。
//!                               存量记录的**形状**迁移是网关职责（与拆家迁移
//!                               同一口径），不做就整家被静默过滤。
//! ```
//!
//! 新账号由插件落库（凭证在 `pluginSecret` 里），`isGuest` 与显示名由插件的
//! `account_view` 提供 —— 本文件的两条判据对它们不参与，也不该参与：网关看不见
//! 插件 secret 里的字段，按自己那份判只会得到「一律 false」。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本文件零 unwrap/expect/panic，不持锁做网络。

use serde_json::Value;

use crate::server::core::account_store::state::StoredAccount;
use crate::server::core::account_store::store_util::token_tail_of;
use crate::server::core::providers::{kind_id, ProviderKind};
use crate::server::logging;

fn chatglm_id() -> &'static str {
    kind_id(ProviderKind::Chatglm)
}

/// 记录是不是**访客**（公开形态的 `isGuest` 与签到范围判定读的同一份事实）。
///
/// ── 为什么不用插件那一份 ────────────────────────────────────
/// 插件记录的 `isGuest` 在 `pluginSecret` 里，网关读不到；`public_account` 会把
/// 插件的 `account_view` 合并进公开形态，所以插件账号的 `isGuest` 由插件自己给。
/// 本函数只服务**存量内置记录**（没有 `pluginSecret`）。
///
/// ── 判据（只认确凿证据）────────────────────────────────────
/// 签到范围是「**确凿是访客才排除**」，所以只认两种证据：
///   1. 记录里的 `isGuest`（添加时判定并落盘的权威值）；
///   2. 缺键时 JWT 里**明说** `is_guest: true`。
/// 读不出任何证据一律给 `false`：让它留在签到范围里，「缺少令牌」这类真实
/// 故障才会被如实报出来。
pub fn guest_of_record(record: &StoredAccount) -> bool {
    if let Some(flag) = record.get("isGuest").and_then(Value::as_bool) {
        return flag;
    }
    // 缺键（手改 / 更早版本写的记录）：退回读 JWT 声明。令牌按**记录契约**
    // 在顶层，`auth.accessToken` 是会话形态的残留，两种都认。
    let token = record
        .get("accessToken")
        .or_else(|| record.get("auth").and_then(|auth| auth.get("accessToken")))
        .and_then(Value::as_str)
        .unwrap_or("");
    guest_claim(token).unwrap_or(false)
}

/// 读 JWT 载荷里的 `is_guest` 声明（不解签，只读；解不出返回 None）。
fn guest_claim(token: &str) -> Option<bool> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64_url_decode(payload)?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    value.get("is_guest").and_then(Value::as_bool)
}

/// base64url（无填充）解码 —— JWT 载荷段的形态。
fn base64_url_decode(text: &str) -> Option<Vec<u8>> {
    let mut buffer: Vec<u8> = Vec::with_capacity(text.len());
    let mut accumulator: u32 = 0;
    let mut bits: u32 = 0;
    for byte in text.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' | b'+' => 62,
            b'_' | b'/' => 63,
            b'=' => continue,
            _ => return None,
        };
        accumulator = (accumulator << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            buffer.push(((accumulator >> bits) & 0xff) as u8);
        }
    }
    Some(buffer)
}

/// 启动迁移：把历史记录里嵌套的 `auth.accessToken` 提升到**顶层**
/// （幂等，可重复调用）。原地改 `accounts`，返回被修复的条数。
///
/// ── 修的是什么（首版接入时留下的存量数据）──────────────────────
/// 首版把令牌写进了嵌套 `auth`，而 `has_credentials()`（转发选路 / 余额查询 /
/// 界面 `hasCredentials` 三处共用的判据）只读顶层 `accessToken`。后果是整家被
/// **静默过滤**：账号导入成功、模型目录也拉得到，但模型管理里看不到这一家、
/// 余额报「没有可用凭证」、点名转发报没有可用账号。
///
/// ── 为什么只搬这两枚令牌 ──────────────────────────────────────
/// 嵌套 `auth` 里当年的写入方只放了 `accessToken` / `refreshToken`，所以其余键
/// 不该被无中生有地搬上来：只做「顶层缺、嵌套有」的那两枚键的补齐。
///
/// ── 何时删掉 `auth` ──────────────────────────────────────────
/// 两枚令牌都已在顶层、且嵌套里没有别的残留时才删掉这个空壳；顶层本来就有值
/// 而嵌套里还留着旧令牌时也删 —— 那种情况下两份值可能不一致，而 `auth` 已经
/// 没有任何读方，留着就是一处「哪份是权威」的歧义。嵌套里还带着别的键时整个
/// `auth` 原样留着：那些键本文件不认识，删掉等于替别的代码丢数据。
///
/// 幂等：抬过一次之后顶层就有了，再跑一遍返回 0、不写库。
pub(crate) fn migrate_nested_tokens(accounts: &mut [StoredAccount]) -> usize {
    let mut healed = 0usize;
    for record in accounts.iter_mut() {
        if record.provider() != chatglm_id() {
            continue;
        }
        // 先把要读的东西**全部克隆成自有值**再改：`record.get` 是共享借用，
        // 与后面的 `record.set` / `record.remove`（可变借用）不能同时活着
        // （与 `clear_rate_limit` / `migrate_rate_limit_keys` 同一处置）。
        let Some(nested) = record.get("auth").and_then(Value::as_object).cloned() else {
            continue;
        };
        // 嵌套里有没有值得搬的令牌（只认这两枚键）
        let nested_token = |key: &str| -> Option<String> {
            nested
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        };
        let nested_access = nested_token("accessToken");
        let nested_refresh = nested_token("refreshToken");
        // 顶层已有的值不动：写入侧已按新约定落过，那份比旧壳里的更可信
        let top_access = record
            .get("accessToken")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("")
            .to_string();
        let top_refresh = record
            .get("refreshToken")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("")
            .to_string();
        let mut changed = false;
        if top_access.is_empty() {
            if let Some(token) = nested_access.as_ref() {
                record.set("accessToken", Value::String(token.clone()));
                changed = true;
            }
        }
        if top_refresh.is_empty() {
            if let Some(token) = nested_refresh.as_ref() {
                record.set("refreshToken", Value::String(token.clone()));
                changed = true;
            }
        }
        // 空壳才删：`auth` 里除了这两枚令牌没别的东西
        let only_tokens = nested
            .keys()
            .all(|key| key == "accessToken" || key == "refreshToken");
        if only_tokens {
            record.remove("auth");
            changed = true;
        }
        if !changed {
            continue;
        }
        // 尾段跟着新令牌重算（原实现也是从令牌算的；顶层一旦补齐，界面读的就是它）
        let tail_source: &str = if !top_access.is_empty() {
            &top_access
        } else {
            nested_access
                .as_deref()
                .or(nested_refresh.as_deref())
                .unwrap_or("")
        };
        if !tail_source.is_empty() {
            record.set("tokenTail", Value::String(token_tail_of(tail_source)));
        }
        record.set_updated_at(logging::now_ms());
        healed += 1;
    }
    healed
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// **令牌必须落在顶层**：`has_credentials()`（转发选路 / 余额查询 / 界面
    /// `hasCredentials` 三处的统一判据）只读顶层 `accessToken`。
    ///
    /// ── 这条测试锁的是真实发生过的故障 ──────────────────────────
    /// 首版把令牌写进嵌套 `auth`（照抄参考实现的读法），后果是
    /// `has_credentials()` 恒假 ⇒ 整家被静默过滤掉：账号导入成功、模型目录也
    /// 拉得到，但模型管理里看不到这一家、余额报「没有可用凭证」、点名转发报
    /// 「没有可用账号」。与 `state.rs::has_officeace_key` 注释里描述的 OfficeAce
    /// 那次同款 —— 改存储键名前先看这条测试。
    #[test]
    fn tokens_must_sit_on_top_level_for_has_credentials() {
        let record = StoredAccount::from_map(
            json!({
                "id": "6ad26bd46976b1c1ab1c2cb2",
                "provider": "chatglm",
                "accessToken": "aaaaaaaaaaaaaaaaaaaa",
                "refreshToken": "bbbbbbbbbbbbbbbbbbbb",
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        assert!(
            record.has_credentials(),
            "顶层 accessToken 必须被 has_credentials 认到（否则整家被静默过滤）"
        );
        assert_eq!(record.access_token(), "aaaaaaaaaaaaaaaaaaaa");
    }

    /// 嵌套 `auth` 形态**不被** `has_credentials()` 认 —— 这就是别再把令牌
    /// 写进那里的原因（迁移负责把存量记录抬上来，见 `migrate_nested_tokens`）。
    #[test]
    fn nested_auth_is_not_recognized_by_has_credentials() {
        let record = StoredAccount::from_map(
            json!({
                "id": "x",
                "provider": "chatglm",
                "auth": { "accessToken": "aaaaaaaaaaaaaaaaaaaa" },
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        assert!(
            !record.has_credentials(),
            "嵌套 auth 不被认（写入侧必须用顶层键）"
        );
    }

    /// 访客标记读两种形态：顶层令牌与历史嵌套令牌里的 JWT 声明都认。
    #[test]
    fn guest_flag_reads_jwt_from_both_shapes() {
        // {"is_guest":true} 的 base64url（无填充）
        let guest = "eyJpc19ndWVzdCI6dHJ1ZX0";
        let top = StoredAccount::from_map(
            json!({ "id": "g", "provider": "chatglm", "accessToken": format!("h.{guest}.s") })
                .as_object()
                .cloned()
                .unwrap_or_default(),
        );
        assert!(guest_of_record(&top), "顶层令牌里的访客声明应当认到");

        let nested = StoredAccount::from_map(
            json!({
                "id": "g2",
                "provider": "chatglm",
                "auth": { "accessToken": format!("h.{guest}.s") },
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        assert!(
            guest_of_record(&nested),
            "历史嵌套形态也要认（免得访客混进签到范围）"
        );

        // 读不出证据一律 false：不放宽成「当访客排除」
        let none = StoredAccount::from_map(
            json!({ "id": "n", "provider": "chatglm", "accessToken": "not-a-jwt" })
                .as_object()
                .cloned()
                .unwrap_or_default(),
        );
        assert!(!guest_of_record(&none), "读不出声明时不能当访客");
    }

    /// **迁移把历史嵌套记录救回来**：抬到顶层之后 `has_credentials()` 才认，
    /// 整家才会重新出现在模型管理与选路里。
    #[test]
    fn migration_promotes_nested_tokens_to_top_level() {
        let mut accounts = vec![StoredAccount::from_map(
            json!({
                "id": "6ad26bd46976b1c1ab1c2cb2",
                "provider": "chatglm",
                "auth": {
                    "accessToken": "nested-access-token",
                    "refreshToken": "nested-refresh-token",
                },
                "deviceId": "c9074ab1-208b-4255-8f71-09cc2ce32278",
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        )];
        // 迁移前：不被认（这正是「整家静默消失」那个症状）
        assert!(!accounts[0].has_credentials());
        let healed = migrate_nested_tokens(&mut accounts);
        assert_eq!(healed, 1, "应当修复 1 条");
        // 迁移后：顶层有值、判据认了、空壳壳也清掉了
        assert!(accounts[0].has_credentials());
        assert_eq!(accounts[0].access_token(), "nested-access-token");
        assert_eq!(accounts[0].refresh_token(), "nested-refresh-token");
        assert!(
            accounts[0].get("auth").is_none(),
            "只装令牌的空壳应当清掉（留着就有两份可能不一致的权威）"
        );
        // 幂等：再跑一遍无事发生、不写库
        assert_eq!(migrate_nested_tokens(&mut accounts), 0);
    }

    /// 迁移**不碰别的家**：判据是本家 provider，别家的 `auth` 原样留着
    /// （那是别家的会话形态，含义与本家这次修的不是一回事）。
    #[test]
    fn migration_ignores_other_providers() {
        let mut accounts = vec![StoredAccount::from_map(
            json!({
                "id": "user-abc",
                "provider": "workbuddy",
                "auth": { "accessToken": "someone-elses-token" },
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        )];
        assert_eq!(migrate_nested_tokens(&mut accounts), 0);
        assert!(
            accounts[0].get("auth").is_some(),
            "别家的 auth 不该被本家迁移动到"
        );
    }

    /// 顶层已有值时不覆盖（写入侧已是新版，那份比旧壳里的更可信），
    /// 但只装令牌的空壳仍会清掉。
    #[test]
    fn migration_keeps_existing_top_level_value() {
        let mut accounts = vec![StoredAccount::from_map(
            json!({
                "id": "x",
                "provider": "chatglm",
                "accessToken": "fresh-top-level-token",
                "auth": { "accessToken": "stale-nested-token" },
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        )];
        assert_eq!(migrate_nested_tokens(&mut accounts), 1);
        assert_eq!(
            accounts[0].access_token(),
            "fresh-top-level-token",
            "顶层已有的新令牌不能被旧壳里的覆盖"
        );
        assert!(accounts[0].get("auth").is_none());
    }

    /// `auth` 里还带着别的键时整个留着：那些键本模块不认识，
    /// 删掉等于替别的代码丢数据。
    #[test]
    fn migration_keeps_auth_shell_with_extra_keys() {
        let mut accounts = vec![StoredAccount::from_map(
            json!({
                "id": "x",
                "provider": "chatglm",
                "auth": {
                    "accessToken": "nested-access-token",
                    "somethingElse": "keep-me",
                },
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        )];
        assert_eq!(migrate_nested_tokens(&mut accounts), 1);
        assert_eq!(accounts[0].access_token(), "nested-access-token");
        assert!(
            accounts[0].get("auth").is_some(),
            "带其它键的 auth 不该被删（会丢数据）"
        );
    }
}
