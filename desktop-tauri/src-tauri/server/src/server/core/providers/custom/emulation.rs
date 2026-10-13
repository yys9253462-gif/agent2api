//! **OpenCode 官方客户端形态伪装**（提供商记录上的 `clientEmulation: "opencode"`）。
//!
//! ── 为什么需要它（上游实测，2026-09-30）────────────────────────
//! `https://opencode.ai/zen/v1` 的**免费档**模型（`*-free` / `big-pickle` 等）
//! 在直连时会回 403：
//! ```text
//! {"type":"error","error":{"type":"FreeTierError","message":"OpenCode's free
//!  tier can only be used from within OpenCode. Download it at
//!  https://opencode.ai and keep building."}}
//! ```
//! 上游用四道校验把免费档锁在官方 CLI 里（对照实现
//! `github.com/rockswang/wild-work/internal/oczen`）：
//!   1. 匿名凭证是**字面量** `public`（`Authorization: Bearer public`）；
//!   2. 会话头须匹配 `ses_<12位小写 hex><14位 Base62>`；
//!   3. 请求体须是「智能体形态」：`stream: true` 且 `tools` 里同时含
//!      `bash` 与 `read` 两个 function；
//!   4. 会话 id 的**低 12 位须落在 `0xFC0` 及以上**（见 [`LOW12_FLOOR`]）。
//! 第 4 条是 2026-10-13 才量出来的：在那之前上游只校验第 2 条的**形状**，
//! 取值随便。它把「按形状伪造」这条路堵成了「按取值伪造」—— 派生值在 48 位
//! 空间里均匀随机，只有 64/4096 落在放行区间。
//!
//! 本模块就实现这四条。**它是有意为之的 opt-in**：只在提供商记录显式写了
//! `clientEmulation: "opencode"` 时才生效（预置卡「OpenCode Zen」会带上
//! 它），手写的家与本网关的其它路径完全不受影响。
//!
//! ── 代价与边界（用它之前要知道的事）─────────────────────────
//!   · 它会**改写用户的请求体**（补两个桩工具、强制 `stream: true`）——
//!     与「自定义家透传」的默认语义相反，所以只在显式开启时做；
//!   · 桩工具是上游校验的产物，不是给模型用的：客户端**原本没有工具**时
//!     一并下发 `tool_choice: "none"`，免得模型真的去调它。客户端自带工具时
//!     保留其 `tool_choice`（改掉会破坏调用工具的客户端），此时桩工具与真
//!     工具并列存在 —— 上游要的就是这个形态；
//!   · 上游随时可能收紧这套校验（历史上有过多次），失效时表现为 403
//!     `FreeTierError`。2026-10-13 收紧的那一次（新增第 4 条取值校验）现场
//!     很好认：**每个账号都 403、且错误文案末尾多了**
//!     `Download it at https://opencode.ai and keep building.` **这一句**。
//!     之所以是「必然 403」而不是「偶尔 403」：模型测试这条路径的会话种子
//!     固定（提示词默认「你好」），派生出的 id 也固定，一次落在区间外就次次
//!     落在区间外。再遇到这类 403，排查顺序是**会话 id 的取值 → 凭证 →
//!     请求体**，最后才怀疑上游改了规则（改规则的判据是「四条的产物都对、
//!     却仍然 403」）。
//!
//! ── 与其它模块的分工 ────────────────────────────────────────
//! 本模块**只回答四件事**：请求体补齐（[`ensure_agent_shape`]）、会话 id 与
//! 它是否合格（[`session_id`] / [`is_acceptable_session_id`]）、伪装头集合
//! （[`apply_headers`]）。发不发、发给谁由 `providers::custom::forward` 决定；
//! 记录字段的读写与校验在 `core::custom_providers`。

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// 提供商记录上的合法取值（与 `custom_providers::CLIENT_EMULATION_OPENCODE` 同字面量；
/// 常量在那边定义是为了让读写与校验在同一处，这里只借用）。
pub use crate::server::core::custom_providers::CLIENT_EMULATION_OPENCODE;

/// 匿名凭证：上游免费档认的字面量（无需注册、无轮换）。
pub const ANONYMOUS_KEY: &str = "public";

/// 伪装成官方 CLI 的 User-Agent（对照实现实测可通过的版本号）
const USER_AGENT: &str = "opencode/1.18.31 (windows amd64; node22)";

/// 免费档校验要求的两个桩工具名（大小写敏感，实测只有小写通过）
const STUB_TOOLS: [&str; 2] = ["bash", "read"];

/// 会话标识的 Base62 字母表（与对照实现逐字一致）
const BASE62: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// 会话种子：**首个 user 消息**的 content 序列化。
///
/// 为什么用它而不是随机值：多轮对话里历史不断增长，取首轮才能让同一场对话
/// 一直映射到同一个会话 —— 上游按会话做提示词缓存亲和，每次换 id 等于每轮
/// 都冷启动。取不到（没有 user 消息 / content 为空）时给空串，调用方退化成
/// 随机会话（单次对话，无亲和需求）。
pub fn conversation_seed(body: &Value) -> String {
    let Some(messages) = body.get("messages").and_then(Value::as_array) else {
        return String::new();
    };
    for message in messages {
        if message.get("role").and_then(Value::as_str) != Some("user") {
            continue;
        }
        let Some(content) = message.get("content") else { continue };
        if content.is_null() {
            continue;
        }
        // 字符串直接取；数组（多模态）与其它形态走序列化 —— 与对照实现
        // （`json.Marshal(m["content"])`）同一口径：**不看内容细节**，
        // 只要同一场对话算出同一个串即可
        let seed = match content {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        if !seed.is_empty() {
            return seed;
        }
    }
    String::new()
}

/// 任意种子 → 上游要求的 `ses_<12位小写 hex><14位 Base62>` 会话 id。
///
/// 合格（形状与取值，见 [`is_acceptable_session_id`]）时**原样返回** ——
/// 保住调用方自带的亲和键；不合格的种子当普通字符串重新派生。
///
/// 派生时低 12 位钉死为 [`LOW12_TAG`]（官方客户端的产出值），其余 36 位与
/// 14 位尾段都来自 `SHA-256(种子)`：**同一场对话仍映射到同一个 id**，上游
/// 按会话做的提示词缓存亲和因此不受影响（这条是取首轮 user 消息当种子的
/// 原因，见 [`conversation_seed`]）。
pub fn session_id(seed: &str) -> String {
    if is_acceptable_session_id(seed) {
        return seed.to_string();
    }
    let seed = if seed.is_empty() {
        // 空种子（没有可派生的对话内容）用随机值：单次会话，无亲和需求，
        // 但形状必须合法 —— 否则上游一律 403
        random_seed()
    } else {
        seed.to_string()
    };
    let digest = Sha256::digest(format!("ses\u{0}{seed}").as_bytes());
    // 低 12 位按官方客户端钉死（理由见 LOW12_TAG 的注释）：直接用散列前
    // 6 字节的话，低 12 位是均匀随机的，只有 64/4096 落在放行区间。
    let mut bits = 0u64;
    for byte in &digest[..6] {
        bits = (bits << 8) | u64::from(*byte);
    }
    let bits = (bits & !0xFFF) | u64::from(LOW12_TAG);
    format!("ses_{bits:012x}{}", base62_fixed(&digest[6..16], 14))
}

/// `ses_` + 12 位小写 hex + 14 位 Base62（上游 2026-09-16 起对其它形状一律 403）
fn valid_session_id(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("ses_") else { return false };
    if rest.len() != 12 + 14 {
        return false;
    }
    let (hex, tail) = rest.split_at(12);
    hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && tail.bytes().all(|byte| BASE62.contains(&byte))
}

/// 会话 id 低 12 位的**取值下界**。
///
/// 2026-10-13 对 `POST /zen/v1/chat/completions` 实测的翻转点：低 12 位
/// `0xFBF`（4031）被拒、`0xFC0`（4032）放行。上游**只看这 12 位** ——
/// `ses_0000000fffff`（数值极小）放行、`ses_ecbcb548a000`（数值极大）被拒，
/// 可见与 id 的数值大小无关；本机历史上 23 个真实 id 的低 12 位全是 `0xFFE`。
/// 5 月、6 月的真实 id 一样放行，阈值在 146 秒内没有移动 —— 这一条**不是**
/// 时间新鲜度校验，是纯粹的位模式校验。
const LOW12_FLOOR: u16 = 0xFC0;

/// 会话 id 低 12 位的**产出值**：官方客户端实测恒为该值，落在放行区间正中。
///
/// 判据用 [`LOW12_FLOOR`]、产出用本值，两者刻意分开：判据是「上游当时认
/// 什么」，本值是「官方客户端发什么」，将来上游若再收紧，改哪里一目了然。
const LOW12_TAG: u16 = 0xFFE;

/// 这个字符串能不能**直接当会话头发出去**：形状（[`valid_session_id`]）与
/// 取值（低 12 位 ≥ [`LOW12_FLOOR`]）都合格才算。
///
/// 拆成两个判据是刻意的：形状是 2026-09 立的规矩、取值是 2026-10 加的，
/// 合成一个函数会让「这个 id 为什么被重算」将来无从追查。
pub fn is_acceptable_session_id(text: &str) -> bool {
    if !valid_session_id(text) {
        return false;
    }
    // `ses_` 之后 12 位是 hex（valid_session_id 已保证是 ASCII），取末 3 位
    // 即低 12 位。用 get 而不是直接切片：本模块的硬约束是不 panic。
    let Some(hex) = text.get(4..16) else { return false };
    let Some(low) = hex.get(9..12).and_then(|tail| u16::from_str_radix(tail, 16).ok()) else {
        return false;
    };
    low >= LOW12_FLOOR
}

/// 随机种子（`getrandom` 失败时退回时间戳 —— 这里只是会话 id 的熵，
/// 不是密钥，退化的后果仅是「同一场对话的亲和键不稳定」，不该让请求失败）
fn random_seed() -> String {
    let mut bytes = [0u8; 16];
    if getrandom::getrandom(&mut bytes).is_ok() {
        return bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    }
    format!("fallback-{}", crate::server::logging::now_ms())
}

/// 定长 Base62（左侧补零到 `width` 位；不足则从低位截断）
fn base62_fixed(bytes: &[u8], width: usize) -> String {
    // 把字节串当一个 128 位大数：逐字节「乘 256 加」并用 62 进制取位。
    // 用 u32 位宽逐位除法实现（不引大整数库）——宽度 14 位 Base62 约 83 bit，
    // 16 字节输入足够，且结果稳定。
    let mut digits = vec![0u32; width];
    for byte in bytes {
        let mut carry = u32::from(*byte);
        for digit in digits.iter_mut().rev() {
            let value = *digit * 256 + carry;
            *digit = value % 62;
            carry = value / 62;
        }
        // 溢出的高位（超出 width 位 Base62 所能表达的范围）直接丢掉：
        // 上游只校验形状与长度，不做唯一性校验
    }
    digits
        .iter()
        .map(|digit| BASE62[(*digit as usize).min(61)] as char)
        .collect()
}

/// 出站头的伪装集合：把官方 CLI 会带的头补齐到 `headers` 上。
///
/// `headers` 的键名沿用本网关的默认头（`Authorization` / `Content-Type`），
/// 同名覆盖 —— 于是**自定义 key 优先**：账号填了 key 就用它（付费模型必须），
/// 没填才用匿名字面量（免费档）。这与 `ProviderQuirks::apply_to` 的合并口径
/// 相同，转发侧先铺默认头、再调本函数，顺序不能反过来。
pub fn apply_headers(headers: &mut Vec<(String, String)>, api_key: &str, session: &str) {
    let credential = if api_key.trim().is_empty() {
        ANONYMOUS_KEY
    } else {
        api_key.trim()
    };
    let pairs = [
        ("Authorization".to_string(), format!("Bearer {credential}")),
        // 官方 CLI 的形状头：缺了「看起来就不像官方客户端」，实测会掉进 403
        ("User-Agent".to_string(), USER_AGENT.to_string()),
        ("x-opencode-client".to_string(), "cli".to_string()),
        ("x-opencode-session".to_string(), session.to_string()),
        ("x-session-affinity".to_string(), session.to_string()),
        ("X-Session-Id".to_string(), session.to_string()),
        ("x-opencode-request".to_string(), random_id("req", 8)),
        ("x-opencode-project".to_string(), random_id("prj", 6)),
    ];
    merge(headers, pairs);
}

/// `prefix` + `_` + 2×size 位 hex（请求 / 项目标识；上游不校验取值，只要形态在）
fn random_id(prefix: &str, size: usize) -> String {
    let mut bytes = vec![0u8; size];
    if getrandom::getrandom(&mut bytes).is_err() {
        // 与 `random_seed` 同一取向：这里只是「形态头」，退化不该让请求失败
        bytes = (0..size).map(|index| (index as u8).wrapping_mul(31)).collect();
    }
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("{prefix}_{hex}")
}

/// 把一对键值按名合并进头表（ASCII 不区分大小写；同名覆盖，其余追加）
fn merge(
    headers: &mut Vec<(String, String)>,
    pairs: impl IntoIterator<Item = (String, String)>,
) {
    for (key, value) in pairs {
        match headers.iter_mut().find(|(name, _)| name.eq_ignore_ascii_case(&key)) {
            Some(slot) => slot.1 = value,
            None => headers.push((key, value)),
        }
    }
}

/// 请求体补齐「智能体形态」：`stream: true` + `tools` 里必须含 `bash` / `read`。
///
/// 返回是否改动了请求体（只给日志用）。三条细节：
///   · `stream` 恒置 true（上游免费档只接受流式；非流式下游由网关聚合器收敛
///     —— 这与编排层对**所有**上游的既有策略一致，不是这里新加的行为）；
///   · 缺哪个桩工具补哪个，已有的同名 function **原样保留**（客户端自己带
///     `bash` 时不重复补，也不会覆盖它的参数定义）；
///   · 客户端**原本没有 tools** 时补 `tool_choice: "none"`：桩工具只为过检，
///     明确禁止模型调用它。客户端自带工具时不动 `tool_choice` —— 那会破坏
///     它本来能用的工具调用。
pub fn ensure_agent_shape(body: &mut Value) -> bool {
    let mut changed = false;
    let Some(object) = body.as_object_mut() else { return false };
    if object.get("stream").and_then(Value::as_bool) != Some(true) {
        object.insert("stream".to_string(), Value::Bool(true));
        changed = true;
    }
    let had_tools = object
        .get("tools")
        .and_then(Value::as_array)
        .is_some_and(|tools| !tools.is_empty());
    let mut tools: Vec<Value> = object
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for name in STUB_TOOLS {
        if tools.iter().any(|tool| tool.get("function").and_then(|function| function.get("name")).and_then(Value::as_str) == Some(name)) {
            continue;
        }
        tools.push(stub_tool(name));
        changed = true;
    }
    object.insert("tools".to_string(), Value::Array(tools));
    if !had_tools {
        object.insert("tool_choice".to_string(), Value::String("none".to_string()));
    }
    changed
}

/// 一个桩工具（参数与描述不参与上游校验，给可辨识的名字是为了让人读日志时
/// 一眼看出它是网关补的）
fn stub_tool(name: &str) -> Value {
    let property = if name == "bash" { "command" } else { "filePath" };
    json!({
        "type": "function",
        "function": {
            "name": name,
            "description": "(internal placeholder — do not call)",
            "parameters": {
                "type": "object",
                "properties": { property: { "type": "string" } },
            },
        },
    })
}
