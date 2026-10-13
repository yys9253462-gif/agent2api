//! 订阅内核（mihomo sidecar）：按需下载、配置生成、进程管理、节点同步。
//!
//! ── 为什么单独一个模块（对照 Clash Verge Rev 的 sidecar 形态）──
//! 订阅节点是 ss / vmess / vless / trojan / hysteria2 这类协议，出网层
//! （`core::egress`）只认 http / socks5 —— 内核负责把订阅转换成本地
//! http/socks 监听器（每节点一个端口），出网层照旧连 `127.0.0.1:端口`，
//! 协议代码一行不改。内核本体是 mihomo 官方二进制（GPL-3.0，按需下载、
//! sha256 校验后落 `{config_dir}/proxy-kernel/`），与本进程是**两个独立
//! 程序**（通过本地 socket 通信，GPL 的「聚合」语义，本项目保持 MIT）。
//!
//! ── 下载按需而不是捆绑 ───────────────────────────────────────
//! 内核 ~20-40MB/平台，捆绑进安装包会让所有用户（多数不用订阅功能）白背
//! 这份体积；按需下载在「订阅」区块第一次点「下载内核」时才拉（GitHub
//! Release 的预编译产物，出口候选与软件更新同一套：设置的更新出口 → 直连
//! → Clash 混合端口兜底），sha256 与 pin 死的值对不上就整体拒装。
//!
//! ── 单 worker 串行（为什么不并发 spawn / kill）────────────────
//! 内核操作（下载 / 重建配置 / 重启 / 刷新）互相踩：并发重启会 spawn 出两个
//! 内核抢同一批端口，并发下载与重启会互相看到半截状态。所有命令进一条
//! mpsc 队列、一个 worker 串行消费；API handler 只投命令 + 读快照，界面用
//! 轮询看进度（下载字节 / 运行状态），不引入第二条推送通道。
//!
//! ── 与代理池的关系 ──────────────────────────────────────────
//! 内核拉到订阅后把节点镜像进池（`proxy_pool::sync_subscription_nodes`，
//! `source: "subscription"` 的只读条目，账号按 id 引用）；订阅删了 / 节点
//! 消失了，镜像条目随之摘掉 —— 与 Clash Verge 镜像（`proxy_pool::sync_clash`）
//! 同一套只读语义。端口映射（节点 → 本地端口）持久在内核目录的
//! `kernel-state.json` 里：重启后端口保持稳定，账号引用不会因重启漂移。

use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde_json::{json, Map, Value};
use tokio::sync::mpsc;

use crate::server::core::proxy_pool;
use crate::server::logging;

pub mod download;
pub mod runtime;

/// 内核版本（pin 死：换版本时改这里并同步 4 份 sha256 —— 不追 latest，
/// 避免上游发版悄悄改行为）
pub const KERNEL_VERSION: &str = "v1.19.32";

/// 内核目录名（`config_dir` 下的子目录，与日志 / updates 同级）
pub const KERNEL_DIR_NAME: &str = "proxy-kernel";

/// 内核目录：`{config_dir}/proxy-kernel/`
pub fn kernel_dir() -> PathBuf {
    crate::paths::config_dir().join(KERNEL_DIR_NAME)
}

/// 当前平台的发布资产（名字 + sha256 pin）。
///
/// 不支持的平台返回 None（界面提示「暂不支持的平台」；内核是 Go 预编译
/// 二进制，官方 Release 只出这几组）。
pub fn asset_for_current_platform() -> Option<(&'static str, &'static str)> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Some((
            "mihomo-windows-amd64-v1.19.32.zip",
            "1ac84e795b5b915446e20139677fb6cc013a0778e876e8d4c96f994750b4c4a3",
        )),
        ("windows", "aarch64") => Some((
            "mihomo-windows-arm64-v1.19.32.zip",
            "7a4f6e58af9a120920935f2150cf642d945085fdb3e753314505b334234e1d1e",
        )),
        ("macos", "x86_64") => Some((
            "mihomo-darwin-amd64-v1.19.32.gz",
            "b7de9fc46af2ca985ecfb302a6362170e04ff2023dee1e82addc54133e669bc9",
        )),
        ("macos", "aarch64") => Some((
            "mihomo-darwin-arm64-v1.19.32.gz",
            "3312a6780652c622890fd4357c6a853bbf865464fd047ac7b7f52dab8de18652",
        )),
        ("linux", "x86_64") => Some((
            "mihomo-linux-amd64-v1.19.32.gz",
            "8451100836c9eda194331c2babfad490b2faf30cebc1a04b0e76fd8ac2d35d10",
        )),
        ("linux", "aarch64") => Some((
            "mihomo-linux-arm64-v1.19.32.gz",
            "9dd862e28b46ff7d775f169cceebc28deccaa0a9e804237d421cd2571e0caba0",
        )),
        _ => None,
    }
}

/// 内核二进制的期望文件名（下载后统一改名到它，config / 状态文件引用固定名）
pub fn kernel_binary_name() -> &'static str {
    if std::env::consts::OS == "windows" {
        "mihomo.exe"
    } else {
        "mihomo"
    }
}

/// 内核是否已安装（二进制存在即视为已装）
pub fn installed() -> bool {
    kernel_dir().join(kernel_binary_name()).exists()
}

// ─── 一条订阅的输入形态 ─────────────────────────────────────

/// worker 命令携带的订阅清单项（api 层从 `core::subscriptions` 的条目构造）。
///
/// 放在输入里而不是让 worker 回读订阅模块：订阅存储（`core::subscriptions`）
/// 依赖本模块的 `ProviderInput` 类型做「已启用清单」的构造，反向依赖会让
/// 两个模块成环 —— 命令带全量输入，worker 不认识订阅存储。
#[derive(Clone, Debug)]
pub struct ProviderInput {
    /// 订阅 id（同时是 mihomo proxy-provider 的名字前缀：`sub_<id>`）
    pub id: String,
    /// 订阅名（代理池节点条目的命名来源）
    pub name: String,
    /// 订阅链接
    pub url: String,
}

/// 内核状态快照（worker 每步操作后更新；status_json 读它）
pub struct StatusSnapshot {
    pub running: bool,
    pub controller_port: Option<u16>,
    pub listener_count: usize,
    pub last_error: Option<String>,
    /// 各订阅最后一次成功同步：`subId → {name, count, at}`
    pub provider_sync: Map<String, Value>,
    pub download: DownloadProgress,
}

/// 下载进度（状态机：idle → downloading → done / error）
pub struct DownloadProgress {
    pub state: &'static str,
    pub received: u64,
    pub total: u64,
    pub error: Option<String>,
}

impl Default for StatusSnapshot {
    fn default() -> Self {
        Self {
            running: false,
            controller_port: None,
            listener_count: 0,
            last_error: None,
            provider_sync: Map::new(),
            download: DownloadProgress {
                state: "idle",
                received: 0,
                total: 0,
                error: None,
            },
        }
    }
}

static STATUS: Mutex<Option<StatusSnapshot>> = Mutex::new(None);

fn patch_status(f: impl FnOnce(&mut StatusSnapshot)) {
    let mut guard = match STATUS.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let snapshot = guard.get_or_insert_with(StatusSnapshot::default);
    f(snapshot);
}

fn with_status<R>(f: impl FnOnce(&StatusSnapshot) -> R) -> R {
    let guard = match STATUS.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let default = StatusSnapshot::default();
    f(guard.as_ref().unwrap_or(&default))
}

// ─── worker（单条命令队列，串行消费）────────────────────────

enum Command {
    /// 重建配置 + 启动 / 重启内核（订阅增删改后都走它）
    Restart(Vec<ProviderInput>),
    /// 强制刷新一条订阅的内容（mihomo 的 provider 强制拉取），节点集合
    /// 变了才重建配置
    Refresh { sub_id: String, providers: Vec<ProviderInput> },
    /// 下载内核；带非空 providers 时下完自动拉起
    Download(Vec<ProviderInput>),
    Stop,
}

static SENDER: OnceLock<mpsc::UnboundedSender<Command>> = OnceLock::new();

/// 投递命令；worker 未启动时静默丢弃（bootstrap 顺序保证先 spawn，
/// 这里只防极端时序 —— 丢了命令的代价是「这次操作没生效」，不是崩）。
fn send(command: Command) {
    if let Some(sender) = SENDER.get() {
        let _ = sender.send(command);
    }
}

/// 起 worker（bootstrap 调用一次；重复调用无副作用 —— sender 只 set 一次）。
pub fn spawn_worker() {
    let (tx, mut rx) = mpsc::unbounded_channel::<Command>();
    if SENDER.set(tx).is_err() {
        return;
    }
    crate::spawn_task(async move {
        while let Some(command) = rx.recv().await {
            // 单 worker 串行：每条命令跑完（含内核重启与节点同步）才收下一条
            execute(command).await;
        }
    });
}

async fn execute(command: Command) {
    match command {
        Command::Restart(providers) => {
            patch_status(|snapshot| {
                snapshot.download.error = None;
            });
            if let Err(error) = runtime::run_restart(&providers).await {
                patch_status(|snapshot| {
                    snapshot.last_error = Some(error.clone());
                    snapshot.running = false;
                });
                logging::log("[Subscription]", &format!("❌ 内核启动失败: {error}"));
            }
        }
        Command::Refresh { sub_id, providers } => {
            if let Err(error) = runtime::run_refresh(&sub_id, &providers).await {
                patch_status(|snapshot| {
                    snapshot.last_error = Some(error.clone());
                });
                logging::log("[Subscription]", &format!("❌ 订阅刷新失败: {error}"));
            }
        }
        Command::Download(providers) => {
            match download::run_download().await {
                Ok(()) => {
                    patch_status(|snapshot| {
                        snapshot.download.state = "done";
                        snapshot.download.error = None;
                        snapshot.last_error = None;
                    });
                    logging::log(
                        "[Subscription]",
                        &format!("✅ 代理内核 {KERNEL_VERSION} 下载完成"),
                    );
                    // 下完自动拉起（有启用的订阅才有意义；没有时用户之后点重启）
                    if !providers.is_empty() {
                        if let Err(error) = runtime::run_restart(&providers).await {
                            patch_status(|snapshot| {
                                snapshot.last_error = Some(error.clone());
                                snapshot.running = false;
                            });
                            logging::log("[Subscription]", &format!("❌ 内核启动失败: {error}"));
                        }
                    }
                }
                Err(error) => {
                    patch_status(|snapshot| {
                        snapshot.download.state = "error";
                        snapshot.download.error = Some(error.clone());
                        snapshot.last_error = Some(error.clone());
                    });
                    logging::log("[Subscription]", &format!("❌ 内核下载失败: {error}"));
                }
            }
        }
        Command::Stop => {
            runtime::stop_kernel().await;
            // 一条启用订阅都不剩 → 订阅节点镜像一并清空（空目标集合 = 摘掉
            // 全部 subscription 条目，见 `proxy_pool::sync_subscription_nodes`）。
            //
            // **必须在停内核这条路径上清**：只有「还有其它启用订阅」时才会走
            // 重启路径顺带同步，删掉最后一条订阅时若不清，节点镜像就留在池里
            // 成了孤儿 —— 归属订阅已不存在，界面上挂着且删不掉（实测踩过）。
            sync_nodes_to_pool(&[]);
            patch_status(|snapshot| {
                snapshot.running = false;
            });
        }
    }
}

// ─── 公开操作（api 层入口）─────────────────────────────────

/// 请求下载内核；`providers` 非空时下完自动拉起。
pub fn request_download(providers: Vec<ProviderInput>) {
    patch_status(|snapshot| {
        snapshot.download.state = "downloading";
        snapshot.download.received = 0;
        snapshot.download.total = 0;
        snapshot.download.error = None;
    });
    send(Command::Download(providers));
}

/// 请求启动 / 重启内核（重建配置：订阅增删改、手动重启都走它）。
pub fn request_restart(providers: Vec<ProviderInput>) {
    send(Command::Restart(providers));
}

/// 请求强制刷新一条订阅的内容。
pub fn request_refresh(sub_id: String, providers: Vec<ProviderInput>) {
    send(Command::Refresh { sub_id, providers });
}

/// 请求停掉内核（一条启用订阅都不剩时调用；空跑的内核只会白占端口）
pub fn request_stop() {
    send(Command::Stop);
}

/// 状态 JSON（GET /api/subscriptions 与列表响应共用）。
///
/// `running` 是快照与**实际探活**的合取：worker 只在自己的操作里更新
/// running，内核进程在两次操作之间崩溃时快照还是 true —— 这里补一次
/// 本地端口探活（connect 到控制端口，毫秒级），状态不符时如实报 false。
pub fn status_json() -> Value {
    with_status(|snapshot| {
        let controller_port = snapshot.controller_port;
        let snapshot_running = snapshot.running;
        let listener_count = snapshot.listener_count;
        let last_error = snapshot.last_error.clone();
        let provider_sync = snapshot.provider_sync.clone();
        let download_state = snapshot.download.state;
        let download_received = snapshot.download.received;
        let download_total = snapshot.download.total;
        let download_error = snapshot.download.error.clone();
        let running = snapshot_running
            && controller_port
                .map(|port| controller_alive(port))
                .unwrap_or(false);
        json!({
            "installed": installed(),
            "version": KERNEL_VERSION,
            "asset": asset_for_current_platform().map(|(name, _)| name),
            "running": running,
            "controllerPort": controller_port,
            "listenerCount": listener_count,
            "kernelDir": kernel_dir().to_string_lossy(),
            "lastError": last_error,
            "providerSync": provider_sync,
            "download": {
                "state": download_state,
                "received": download_received,
                "total": download_total,
                "error": download_error,
            },
        })
    })
}

/// 控制端口探活（本地 TCP connect，超时 300ms）
fn controller_alive(port: u16) -> bool {
    let address = format!("127.0.0.1:{port}");
    std::net::ToSocketAddrs::to_socket_addrs(&address)
        .ok()
        .and_then(|mut addresses| addresses.next())
        .map(|address| TcpStream::connect_timeout(&address, Duration::from_millis(300)).is_ok())
        .unwrap_or(false)
}

/// 停机钩子（backend::shutdown 调）：best-effort 杀掉内核子进程。
///
/// worker 可能正阻塞在下一条命令上（收不到 Stop 也要杀），所以这里直接
/// 碰 runtime 里的子进程句柄，不走命令队列。
pub fn shutdown() {
    runtime::kill_child();
    logging::log("[Subscription]", "🛑 已停止代理内核");
}

// ─── 给 runtime / download 的共享小件 ───────────────────────

/// 把节点集合镜像进代理池（runtime 在内核同步完成后调用）。
///
/// 目标类型定义在 proxy_pool（`SubscriptionNodeTarget`）：本模块 → proxy_pool
/// 单向依赖，proxy_pool 不反过来认识内核（订阅条目的端口存在条目自身里，
/// 解析不需要查内核状态）。
pub fn sync_nodes_to_pool(targets: &[proxy_pool::SubscriptionNodeTarget]) {
    match proxy_pool::sync_subscription_nodes(targets) {
        Ok(changes) if changes > 0 => {
            logging::log(
                "[Subscription]",
                &format!("🔄 已同步订阅节点到代理池（{changes} 项变更）"),
            );
        }
        Ok(_) => {}
        Err(error) => logging::log("[Subscription]", &format!("❌ 节点同步代理池失败: {error}")),
    }
}

/// Arc 包装的控制器端口记录（runtime 写快照用）
pub fn record_running(controller_port: Option<u16>, listener_count: usize, error: Option<String>) {
    patch_status(|snapshot| {
        snapshot.running = error.is_none() && controller_port.is_some();
        snapshot.controller_port = controller_port;
        snapshot.listener_count = listener_count;
        snapshot.last_error = error;
    });
}

/// 记录一次订阅同步成功（runtime 的 provider 轮询后调用）
pub fn record_provider_sync(sub_id: &str, sub_name: &str, count: usize) {
    patch_status(|snapshot| {
        snapshot.provider_sync.insert(
            sub_id.to_string(),
            json!({
                "name": sub_name,
                "count": count,
                "at": logging::now_ms(),
            }),
        );
    });
}

/// 记一条内核运行日志（runtime / download 共用；控制台 + 运行日志一条口径）
pub fn log_line(message: &str) {
    logging::log("[Subscription]", message);
}

/// 记下载进度（download 在流式落盘时调用，界面轮询渲染）。
pub fn record_download_progress(received: u64, total: u64) {
    patch_status(|snapshot| {
        snapshot.download.received = received;
        snapshot.download.total = total;
    });
}
