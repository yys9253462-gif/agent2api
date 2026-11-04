//! AutoClaw（智谱 autoglm）的**地区身份**（网关侧只剩这一个模块）。
//!
//! ── 为什么只剩 `region` ─────────────────────────────────────
//! AutoClaw 的实现已**整体**搬到提供商插件（数据面 + 账号面 + 登录面）：
//!
//! ```text
//!   转发 / 请求头 / 三道闸        plugins/autoclaw/adapter 与 mod.rs
//!   凭证来源 / safeStorage 解密   plugins/autoclaw/credentials.rs + crypto.rs
//!   token 续期（单飞 + 降级）      plugins/autoclaw/refresh.rs
//!   模型路由 + 远程目录            plugins/autoclaw/models.rs + catalog.rs
//!   积分 / 订阅                    plugins/autoclaw/balance.rs
//!   签到 / 活动福利                plugins/autoclaw/checkin.rs + promotion.rs
//!   短信登录（国内版）             plugins/autoclaw/login.rs
//!   OAuth 两跳（国际版）           plugins/autoclaw/oauth.rs
//!   登记端口回调监听器             plugins/autoclaw/callback_server.rs
//!   网页登录流程表（轮询型）        plugins/autoclaw/web_login.rs
//! ```
//!
//! 网关这边只剩两件**本地**的事：
//!   1. 地区 ↔ provider id 的映射（落盘契约 + 界面分组依据，见 [`region`]）；
//!   2. 存量账号记录的公开形态投影（`core::account_store::autoclaw_accounts`）——
//!      那些记录没有 `pluginSecret`，走不到插件的 `account_view`。
//!
//! ── 拆家的目的（别把实现加回来）─────────────────────────────
//! 反向代理的业务实现（UA 伪装、客户端指纹、上游协议细节、官方客户端登录链）
//! 不留在主仓库：它带来的风险由主仓库承担，收益却属于网关之外。加回任何一条
//! 都会把那个风险带回来 —— 插件目录是**唯一**实现点。
//!
//! ── 平台约束 ───────────────────────────────────────────────
//! auth.json 来源（safeStorage 解密）仅 Windows 可用 —— 那条链现在在插件里，
//! 网关侧不再关心。本模块与平台无关。

pub mod region;

pub use region::Region;
