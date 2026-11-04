//! 侧边栏布局（分组 / 顺序 / 显隐）的持久化。
//!
//! ── 为什么从 localStorage 搬进库 ────────────────────────────
//! 早先它是纯前端偏好（与主题 / 缩放 / 语言同族），只存浏览器 localStorage。
//! 那条取舍成立的前提是「只影响本机这台设备的界面排布，不需要跨设备同步」——
//! 但本产品有两个界面入口（桌面壳的 `tauri://localhost` 与网关托管的
//! `http://127.0.0.1:<port>`），**两者 origin 不同、localStorage 天然隔离**，
//! 于是在 WebUI 里隐藏的菜单在客户端里照旧显示，用户看到的是「设置没生效」。
//!
//! 落库之后库是**唯一事实来源**，两端读同一份；localStorage 降级成首帧缓存
//! （见 `ui/sidebar-layout.js` 的模块头 —— 那里解释了为什么不能把首帧改成
//! 异步渲染）。
//!
//! ── 为什么后端不校验布局结构 ────────────────────────────────
//! 布局的合法形状（哪些菜单键存在、固定项不可隐藏、分组至少要有一个、
//! 新菜单往哪补位）由前端 `ui/sidebar-layout.js` 的 `MENUS` 表定义 ——
//! 那张表随版本增删菜单，而**插件页的菜单键是运行期才有的**
//! （`plugin:<pack>:<page>`，见 `core::plugin_host`）。后端抄一份校验规则
//! 只会多一处必然漂移的副本：前端加一个菜单，后端就把它当非法键丢掉。
//!
//! 所以本模块把布局当**不透明的 JSON 文档**存：写入只要求顶层是对象，
//! 读出的原文原样交还，规整（`normalizeLayout`）全部在前端做。这与
//! `agentProfiles` 的处理取向不同（那边后端认识 `Profile` 结构），原因是
//! 侧边栏布局的形状由**界面**拥有，而不是由网关拥有。
//!
//! ── 存储形态 ────────────────────────────────────────────────
//! `kv` 表一个固定键，整份 JSON 进出（与 `proxySettings` / `agentProfiles`
//! 同类零散状态）。不新增表，也就不动 schema 版本。键名登记在
//! `db::schema::RESERVED_KV_KEYS` —— 漏登记会让用户改一次 API Key 就把
//! 布局当「配置里已删掉的键」清掉。

use serde_json::Value;

use crate::server::access;

/// `kv` 表里的键名（保留键，见 `db::schema::RESERVED_KV_KEYS`）
pub const KV_KEY: &str = "sidebarLayout";

/// 读整份布局。`None` = 库里没存过（前端回落默认布局）。
///
/// 解析失败也返回 `None`：库里那份读不懂时，前端的默认布局比一份坏数据好 ——
/// 坏数据会让 `normalizeLayout` 丢掉全部未知键，画出一个残缺的侧边栏。
///
/// 空对象同样按「没存过」处理：`save(None)`（恢复默认）落的就是 `{}`，
/// 若在这里返回 `Some({})`，接口就会回 `{layout: {}}` 而不是文档承诺的
/// `{layout: null}` —— 前端两种都能消化（`normalizeLayout({})` 正好落到
/// `defaultLayout()`），但契约该只有一种说法。
pub fn load() -> Option<Value> {
    let text = access::kv_get(KV_KEY)?;
    let value = serde_json::from_str::<Value>(&text).ok()?;
    // 顶层不是对象 = 不是布局（数组 / 标量 / null）—— 当作没存过
    let object = value.as_object()?;
    (!object.is_empty()).then_some(value)
}

/// 整份写回。`None` = 清除（回落默认布局）。
///
/// 与 `core::agents::save_client` 同一取向：整键重写（布局是整体读写单元，
/// 拆键会让一次写入变成多行更新）。写成功显式通知云同步 —— 保留键不走
/// `config::update`，那条通知路径覆盖不到它。
pub fn save(layout: Option<&Value>) -> Result<(), String> {
    match layout {
        Some(value) => {
            if !value.is_object() {
                return Err("侧边栏布局必须是 JSON 对象".to_string());
            }
            let text = serde_json::to_string(value)
                .map_err(|error| format!("布局序列化失败: {error}"))?;
            access::kv_put(KV_KEY, &text);
        }
        // 清除：写空对象而不是删行 —— `kv_put` 没有删除口，而空对象经
        // `normalizeLayout` 正好落到默认布局（`groups` 为空 → `defaultLayout()`），
        // 语义与「没存过」一致，且不必为此新开一条 DELETE 路径。
        None => access::kv_put(KV_KEY, "{}"),
    }
    // `sidebarLayout` 在云同步范围内（见 `cloud_sync::snapshot` 的
    // `SYNCED_RESERVED_KEYS`），保留键不经过 `config::update`，显式通知。
    crate::server::core::cloud_sync::auto_sync::notify_change();
    Ok(())
}
