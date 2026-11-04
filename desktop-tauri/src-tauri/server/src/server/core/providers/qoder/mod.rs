//! Qoder：**两个地区 = 两家 provider** 的身份定义。
//!
//! **实现已整体搬到提供商插件**（`agent2api-providers`，不随本仓库发布）：
//! COSY 签名与请求体编码、SSE 信封拆解、设备授权登录、续期、额度、每日签到、
//! 设备风控（UMID 组件）、模型目录与会话式转发全在那边。网关只留身份与配置：
//!
//! ```text
//!   endpoints.rs   地区 ↔ provider id ↔ ProviderKind 的互查、展示字段
//!   账号面         `account_store::qoder_accounts`（存量记录的公开形态投影）
//!   风控组件状态   `api::qoder_umid`（问插件的 `admin.action`）
//! ```
//!
//! ── 为什么走会话式转发（在插件里）────────────────────────────
//! 无状态路径假设「请求体由通用层序列化后原样发出、响应是 OpenAI 协议的 SSE」。
//! Qoder 两条都不满足：请求体要先编码再签名，响应还要拆一层信封 —— 因此
//! 插件实现 `conversation`，把「构造 → 发送 → 翻译」整条链收进去。账号选路、
//! 限额冷却、telemetry 记账仍由网关的编排层统一负责。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本模块零 unwrap/expect/panic。

pub mod endpoints;

pub use endpoints::Region;
