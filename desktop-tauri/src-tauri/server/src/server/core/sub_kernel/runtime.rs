//! 内核运行时：配置生成、进程 spawn/kill、mihomo 控制器调用、节点同步。
//!
//! ── 与 mihomo 的全部交互只有两条本地通道 ──────────────────────
//!   ① 配置文件：`{kernel_dir}/config.yaml`（本模块生成，serde_yaml 序列化
//!      —— 节点名是用户提供的数据，字符串拼接会坏 YAML，一律走结构化序列化）；
//!   ② 控制器 REST：`http://127.0.0.1:<port>`（Bearer secret，绑定 127.0.0.1
//!      —— 只本机可达；secret 随机生成、落在 kernel-state.json 里，与内核
//!      进程一一对应）。
//! 进程是独立程序（GPL「聚合」边界，见 mod.rs 模块头），不做任何进程内嵌入。
//!
//! ── 启动流程（run_restart）─────────────────────────────────
//!   ① 清理上次运行遗留的内核进程（pid 从状态文件里来，控制器以我们的
//!      secret 响应确认「是我们的」才杀 —— pid 复用会误伤无关进程）；
//!   ② 读 provider 缓存文件拿节点名（缓存缺失 = 该订阅还没拉到过，先不给它
//!      生成监听器 —— 内核启动时自己会拉，拉到后第 ⑤ 步补一轮）；
//!   ③ 端口分配：控制端口与节点端口从状态文件里沿用，缺了才现分配（绑定
//!      探测挑空闲口）；重启后端口保持稳定，账号引用不漂移；
//!   ④ 生成配置 → 杀旧进程 → spawn 新进程 → 等控制器就绪；
//!   ⑤ 轮询控制器拿各 provider 的节点清单 → 节点集合与配置不一致（首次
//!      拉到 / 订阅更新过）时重建配置 + 重启一次；
//!   ⑥ 节点镜像进代理池（`proxy_pool::sync_subscription_nodes`）。
//!
//! ── 失败形态 ────────────────────────────────────────────────
//! spawn 失败 / 控制器起不来 / 订阅拉取失败都返回 Err（worker 记进快照的
//! last_error）；某一家订阅拉不到不影响其它家的监听器（错误文案里点名是
//! 哪一家，内核照常带着其余节点跑）。

use std::collections::HashSet;
use std::net::{SocketAddr, TcpListener};
use std::process::Stdio;
use std::time::Duration;

use serde_json::{json, Map, Value};
use serde_yaml::Value as Yaml;

use crate::server::core::proxy_pool::SubscriptionNodeTarget as NodeTarget;
use crate::server::core::sub_kernel::{
    log_line, record_provider_sync, record_running, sync_nodes_to_pool, ProviderInput,
    KERNEL_VERSION,
};
use crate::server::logging;

/// 控制端口候选范围（避开 Clash Verge 默认的 7897 与常见 9090）
const CONTROLLER_PORT_BASE: u16 = 19090;
const CONTROLLER_PORT_END: u16 = 19099;
/// 节点监听端口候选起点（控制端口段之外，向上顺延）
const LISTENER_PORT_BASE: u16 = 19101;
const LISTENER_PORT_END: u16 = 19200;
/// 等控制器就绪 / 等 provider 拉取的总时长
const CONTROLLER_WAIT_MS: u64 = 10_000;
const PROVIDER_WAIT_MS: u64 = 25_000;
/// 状态文件名（内核目录下：{version, controllerPort, secret, ports, pid}）
const STATE_FILE: &str = "kernel-state.json";

/// 内核子进程句柄（kill_on_drop(true)：句柄 drop 时进程被杀 —— 重启与停机
/// 都是「取走句柄 → drop」，不需要额外的杀进程入口；drop 后的收尾由
/// spawn_task 里的 wait 完成，不留僵尸）。
static CHILD: std::sync::Mutex<Option<tokio::process::Child>> = std::sync::Mutex::new(None);

fn lock_child() -> std::sync::MutexGuard<'static, Option<tokio::process::Child>> {
    match CHILD.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// 杀掉内核子进程（停机钩子与重启共用；没有子进程时是 no-op）。
///
/// 可能从**同步上下文**调（backend::shutdown 不在 async 里）：
/// `start_kill()` 是同步的（只发信号），僵尸收尾交给运行时 wait。
pub fn kill_child() {
    let Some(mut child) = lock_child().take() else {
        return;
    };
    let _ = child.start_kill();
    crate::spawn_task(async move {
        let _ = child.wait().await;
    });
}

// ─── 状态文件（端口注册表 + secret + pid）──────────────────

fn state_file_path() -> std::path::PathBuf {
    crate::server::core::sub_kernel::kernel_dir().join(STATE_FILE)
}

fn read_state() -> Value {
    std::fs::read_to_string(state_file_path())
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .unwrap_or_else(|| json!({}))
}

fn save_state(state: &Value) -> Result<(), String> {
    let dir = crate::server::core::sub_kernel::kernel_dir();
    std::fs::create_dir_all(&dir).map_err(|error| format!("创建内核目录失败: {error}"))?;
    let text =
        serde_json::to_string_pretty(state).map_err(|error| format!("编码内核状态失败: {error}"))?;
    std::fs::write(state_file_path(), text).map_err(|error| format!("写入内核状态失败: {error}"))
}

/// 从状态里读「节点键 → 端口」的映射（nodeKey = `<订阅id>::<节点名>`）
fn state_ports(state: &Value) -> Map<String, Value> {
    state
        .get("ports")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

/// 往状态对象里写一个键（Value 顶层没有 insert，走 as_object_mut）
fn state_insert(state: &mut Value, key: &str, value: Value) {
    if let Some(object) = state.as_object_mut() {
        object.insert(key.to_string(), value);
    }
}

/// 状态里的端口号（数字 1-65535；serde_json 没有 as_u16，走 as_u64 收窄）
fn json_u16(value: &Value) -> Option<u16> {
    value
        .as_u64()
        .filter(|number| (1..=65535).contains(number))
        .map(|number| number as u16)
}

// ─── 端口分配 ──────────────────────────────────────────────

/// 端口是否空闲（绑定探测：bind 成功后立即 drop）
fn port_free(port: u16) -> bool {
    let Ok(address) = format!("127.0.0.1:{port}").parse::<SocketAddr>() else {
        return false;
    };
    TcpListener::bind(address).is_ok()
}

/// 从 [from, end] 里挑第一个空闲端口（`taken` 是已注册 / 已占用的集合）
fn alloc_port(taken: &HashSet<u16>, from: u16, end: u16) -> Option<u16> {
    (from..=end).find(|port| !taken.contains(port) && port_free(*port))
}

// ─── provider 缓存读取 ─────────────────────────────────────

/// 读一个 provider 缓存文件里的节点名（缺失 / 解析失败 / 没有 proxies 都给空表
/// —— 内核启动时会自己拉，拉到后第 ⑤ 步补监听器）。
fn read_provider_nodes(provider_id: &str) -> Vec<String> {
    let path = crate::server::core::sub_kernel::kernel_dir()
        .join("providers")
        .join(format!("sub_{provider_id}.yaml"));
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(parsed) = serde_yaml::from_str::<Yaml>(&text) else {
        return Vec::new();
    };
    let Some(items) = parsed.get("proxies").and_then(Yaml::as_sequence) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| item.get("name").and_then(Yaml::as_str).map(str::to_string))
        .collect()
}

// ─── 控制器调用（本地 REST，Bearer secret）─────────────────

/// 控制器 GET（响应失败一律 None —— 轮询语义里「还没就绪」与「失败」同路）
async fn controller_get(port: u16, secret: &str, path: &str) -> Option<Value> {
    let response = controller_client()
        .get(format!("http://127.0.0.1:{port}{path}"))
        .header("Authorization", format!("Bearer {secret}"))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.json::<Value>().await.ok()
}

/// 控制器 PUT（强制刷新 provider 用）
async fn controller_put(port: u16, secret: &str, path: &str) -> Result<(), String> {
    let response = controller_client()
        .put(format!("http://127.0.0.1:{port}{path}"))
        .header("Authorization", format!("Bearer {secret}"))
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|error| format!("控制器请求失败: {error}"))?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("控制器返回 {}", response.status().as_u16()))
    }
}

/// 控制器专用客户端：**显式 no_proxy** —— 控制器是本机回环，绝不能被环境里的
/// HTTPS_PROXY 悄悄改写（否则控制器调用会打到环境代理上并失败）。与
/// egress 的直连客户端同一手法，但那里缓存键带超时设置、代理可配，这里
/// 独立一份免得与出网缓存互相干扰。
fn controller_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// 轮询控制器拿「全部 provider 的节点清单」：`{subId: [节点名]}`。
///
/// 内核启动后 provider 是异步拉的（缓存缺失时才现拉），所以给
/// `wait_ms` 的窗口轮询；有任一 provider 出现节点就提前收手
/// （各家独立，不互相等）。
async fn poll_providers(
    port: u16,
    secret: &str,
    provider_ids: &[String],
    wait_ms: u64,
) -> Map<String, Value> {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(wait_ms);
    let mut latest: Map<String, Value> = Map::new();
    while tokio::time::Instant::now() < deadline {
        if let Some(value) = controller_get(port, secret, "/providers/proxies").await {
            if let Some(providers) = value.get("providers").and_then(Value::as_object) {
                for provider_id in provider_ids {
                    let Some(provider) = providers.get(&format!("sub_{provider_id}")) else {
                        continue;
                    };
                    let names: Vec<Value> = provider
                        .get("proxies")
                        .and_then(Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(|item| item.get("name").and_then(Value::as_str).map(|name| json!(name)))
                                .collect()
                        })
                        .unwrap_or_default();
                    latest.insert(provider_id.clone(), Value::Array(names));
                }
            }
            // 至少一家拿到了节点就收手 —— 拉取失败的留给下一次 restart 再试
            let any_nodes = latest
                .values()
                .any(|value| value.as_array().map(|items| !items.is_empty()).unwrap_or(false));
            if any_nodes {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(800)).await;
    }
    latest
}

// ─── 配置生成 ──────────────────────────────────────────────

/// 生成 mihomo 配置（结构化 → serde_yaml 序列化，节点名不做字符串拼接）。
///
/// 刻意**不开 TUN、不接管系统代理、没有分流规则**（只有一条 MATCH,DIRECT
/// 兜底）：本内核只服务「每节点一个本地监听器」这一件事，系统代理 / 全局
/// 分流是用户自己的 Clash Verge 的事，这里越界只会打架。也不带 geo 规则
/// —— 用不到 geoip/geosite，内核不会去下载那几十 MB 的地理库。
///
/// 监听器名用节点键的短哈希（`wbn_<hash>`）：节点名含任意字符，做监听器
/// 名不安全；哈希稳定（同节点同名），与监听器配置一一对应。
fn build_config(
    providers: &[ProviderInput],
    controller_port: u16,
    secret: &str,
    listeners: &[NodeTarget],
) -> Result<String, String> {
    let mut proxy_providers = Map::new();
    for provider in providers {
        proxy_providers.insert(
            format!("sub_{}", provider.id),
            json!({
                "type": "http",
                "url": provider.url,
                "interval": 86400,
                "path": format!("providers/sub_{}.yaml", provider.id),
                "health-check": {
                    "enable": true,
                    "url": "https://www.gstatic.com/generate_204",
                    "interval": 1800,
                },
            }),
        );
    }
    let listener_list: Vec<Value> = listeners
        .iter()
        .map(|target| {
            json!({
                "name": format!("wbn_{}", short_hash(&node_key(&target.provider_id, &target.node))),
                "type": "mixed",
                "port": target.port,
                "listen": "127.0.0.1",
                "proxy": target.node,
            })
        })
        .collect();
    let config = json!({
        "mode": "rule",
        "log-level": "warning",
        "ipv6": false,
        "external-controller": format!("127.0.0.1:{controller_port}"),
        "secret": secret,
        "profile": { "store-selected": false },
        "proxy-providers": proxy_providers,
        "rules": ["MATCH,DIRECT"],
        "listeners": listener_list,
    });
    serde_yaml::to_string(&config).map_err(|error| format!("编码内核配置失败: {error}"))
}

/// 节点键（端口注册表与代理池条目共用的稳定标识）
fn node_key(provider_id: &str, node: &str) -> String {
    format!("{provider_id}::{node}")
}

/// 短哈希（监听器名用，只求稳定不求密码学强度）
fn short_hash(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{hash:08x}")
}

// ─── 内核启动 / 重启 ───────────────────────────────────────

/// 重建配置 + 启动（或重启）内核 + 同步节点（详见模块头「启动流程」）。
pub async fn run_restart(providers: &[ProviderInput]) -> Result<(), String> {
    if !crate::server::core::sub_kernel::installed() {
        return Err("代理内核未安装（先在「订阅」区块下载内核）".to_string());
    }
    let dir = crate::server::core::sub_kernel::kernel_dir();
    std::fs::create_dir_all(dir.join("providers"))
        .map_err(|error| format!("创建内核目录失败: {error}"))?;

    // ① 遗留进程清理（上次崩溃留下的内核还占着控制端口；pid 复用有
    //    secret 验证挡着，不会误伤）
    cleanup_stale_kernel().await;

    // ② provider 缓存里的节点名（可能为空 —— 首次拉取在第 ⑤ 步补）
    let mut provider_nodes: Vec<(String, Vec<String>)> = Vec::new();
    for provider in providers {
        provider_nodes.push((provider.id.clone(), read_provider_nodes(&provider.id)));
    }

    // ③ 端口分配：状态文件里沿用，缺了才现分配
    let mut state = read_state();
    let secret = {
        let existing = state.get("secret").and_then(Value::as_str).unwrap_or("");
        if existing.is_empty() {
            let generated = random_secret();
            state_insert(&mut state, "secret", json!(generated));
            generated
        } else {
            existing.to_string()
        }
    };
    let controller_port = ensure_controller_port(&mut state, &secret).await?;
    let targets = allocate_listeners(providers, &state_ports(&state), &provider_nodes)?;
    write_ports(&mut state, &targets);
    state_insert(&mut state, "version", json!(KERNEL_VERSION));
    save_state(&state)?;

    // ④ 写配置 → 杀旧 → spawn → 等控制器
    restart_with_config(providers, controller_port, &secret, &targets).await?;

    // ⑤ 轮询 provider：节点集合与配置不一致时重建 + 重启一次
    let provider_ids: Vec<String> = providers.iter().map(|provider| provider.id.clone()).collect();
    let live = poll_providers(controller_port, &secret, &provider_ids, PROVIDER_WAIT_MS).await;
    let mut final_targets = targets;
    if nodes_changed(&final_targets, &live) {
        let live_nodes: Vec<(String, Vec<String>)> = provider_ids
            .iter()
            .map(|id| {
                let names = live
                    .get(id)
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default();
                (id.clone(), names)
            })
            .collect();
        let next_targets = allocate_listeners(providers, &state_ports(&state), &live_nodes)?;
        let mut state = read_state();
        write_ports(&mut state, &next_targets);
        save_state(&state)?;
        restart_with_config(providers, controller_port, &secret, &next_targets).await?;
        final_targets = next_targets;
    }

    // ⑥ 节点镜像进代理池 + 记录各订阅的同步结果
    sync_nodes_to_pool(&final_targets);
    for provider in providers {
        let count = final_targets
            .iter()
            .filter(|target| target.provider_id == provider.id)
            .count();
        record_provider_sync(&provider.id, &provider.name, count);
        if count == 0 {
            log_line(&format!(
                "⚠️ 订阅「{}」还没有节点（链接拉取失败或内容为空，可点「立即更新」重试）",
                provider.name
            ));
        }
    }
    Ok(())
}

/// 把节点的端口映射写回状态（重启后端口稳定，账号引用不漂移）
fn write_ports(state: &mut Value, targets: &[NodeTarget]) {
    let mut port_map = state_ports(state);
    for target in targets {
        port_map.insert(node_key(&target.provider_id, &target.node), json!(target.port));
    }
    state_insert(state, "ports", Value::Object(port_map));
}

/// 杀旧 + 写配置 + spawn + 等控制器就绪。
async fn restart_with_config(
    providers: &[ProviderInput],
    controller_port: u16,
    secret: &str,
    targets: &[NodeTarget],
) -> Result<(), String> {
    let dir = crate::server::core::sub_kernel::kernel_dir();
    let config = build_config(providers, controller_port, secret, targets)?;
    std::fs::write(dir.join("config.yaml"), &config)
        .map_err(|error| format!("写入内核配置失败: {error}"))?;

    // 杀旧进程（重启语义；首次启动时是 no-op）
    kill_child();

    let binary = dir.join(crate::server::core::sub_kernel::kernel_binary_name());
    if !binary.exists() {
        return Err("代理内核未安装（先在「订阅」区块下载内核）".to_string());
    }
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("kernel.log"))
        .map_err(|error| format!("打开内核日志失败: {error}"))?;
    let log_err = log_file
        .try_clone()
        .map_err(|error| format!("打开内核日志失败: {error}"))?;
    let mut child = tokio::process::Command::new(&binary)
        .arg("-d")
        .arg(&dir)
        .arg("-f")
        .arg(dir.join("config.yaml"))
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_err))
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("启动内核进程失败: {error}"))?;

    // 等控制器就绪（内核启动 ~100ms，给 10s 的宽限）；起完就崩时把退出
    // 状态带出来
    let deadline = tokio::time::Instant::now() + Duration::from_millis(CONTROLLER_WAIT_MS);
    loop {
        if controller_get(controller_port, secret, "/version").await.is_some() {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            let _ = child.start_kill();
            return Err(format!(
                "内核控制器未就绪（端口 {controller_port}，详见 kernel.log）"
            ));
        }
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!("内核进程退出（{status}，详见 kernel.log）"));
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    // 持久 pid（下一次启动识别遗留进程用）+ 记录句柄（kill_on_drop 杀它）
    let pid = child.id();
    *lock_child() = Some(child);
    let mut state = read_state();
    if let Some(pid) = pid {
        state_insert(&mut state, "pid", json!(pid));
        save_state(&state)?;
    }

    record_running(Some(controller_port), targets.len(), None);
    log_line(&format!(
        "✅ 代理内核 {KERNEL_VERSION} 已启动（控制端口 {controller_port}，{} 个节点监听器）",
        targets.len()
    ));
    Ok(())
}

/// 停掉内核（worker 的 Stop 分支用；等待进程退出再返回）
pub async fn stop_kernel() {
    let Some(mut child) = lock_child().take() else {
        record_running(None, 0, None);
        return;
    };
    let _ = child.start_kill();
    let _ = child.wait().await;
    record_running(None, 0, None);
    log_line("🛑 已停止代理内核");
}

// ─── 刷新与遗留进程 ────────────────────────────────────────

/// 强制刷新一条订阅的内容：控制器 PUT（provider 强制拉取）→ 轮询节点 →
/// 集合变了才重建配置 + 重启（端口注册表保持稳定）。
pub async fn run_refresh(sub_id: &str, providers: &[ProviderInput]) -> Result<(), String> {
    let state = read_state();
    let Some(controller_port) = state.get("controllerPort").and_then(json_u16) else {
        return Err("内核未运行（先启动内核）".to_string());
    };
    let Some(secret) = state.get("secret").and_then(Value::as_str) else {
        return Err("内核状态缺少控制器密钥".to_string());
    };
    if !controller_alive_port(controller_port) {
        return Err("内核未运行（先启动内核）".to_string());
    }
    let provider = providers
        .iter()
        .find(|provider| provider.id == sub_id)
        .ok_or_else(|| "订阅不存在".to_string())?;
    controller_put(
        controller_port,
        secret,
        &format!("/providers/proxies/sub_{sub_id}"),
    )
    .await?;
    log_line(&format!("🔄 已强制刷新订阅「{}」", provider.name));

    // 拉取后的节点清单（给几秒的窗口让 provider 写缓存）
    let live = poll_providers(controller_port, secret, &[sub_id.to_string()], 8_000).await;
    let names: Vec<String> = live
        .get(sub_id)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let count = names.len();
    record_provider_sync(sub_id, &provider.name, count);
    if count == 0 {
        return Err(format!(
            "订阅「{}」刷新后仍然没有节点（检查链接或流量是否用尽）",
            provider.name
        ));
    }

    // 节点集合变了 → 重建配置 + 重启（新节点分新端口，消失的摘掉）
    let current: HashSet<String> = read_provider_nodes(sub_id).into_iter().collect();
    let next: HashSet<String> = names.iter().cloned().collect();
    if current != next {
        run_restart(providers).await?;
    }
    Ok(())
}

/// 启动前的遗留进程清理：状态文件里的 pid 还活着且确实是本内核
/// （控制器以我们的 secret 响应）才杀 —— pid 复用会误伤无关进程。
///
/// 桌面端正常退出走 backend::shutdown（杀内核），这里兜「上次崩溃 /
/// 被任务管理器强杀」的情况。
pub async fn cleanup_stale_kernel() {
    let state = read_state();
    let (Some(pid), Some(controller_port), Some(secret)) = (
        state.get("pid").and_then(Value::as_u64),
        state.get("controllerPort").and_then(json_u16),
        state.get("secret").and_then(Value::as_str),
    ) else {
        return;
    };
    if !controller_alive_port(controller_port) {
        return; // 内核已死，pid 记录过期，无需处理
    }
    // 控制器活着：确认是我们的（secret 对上）才按 pid 杀
    if controller_get(controller_port, secret, "/version").await.is_none() {
        return;
    }
    if kill_by_pid(pid) {
        logging::log(
            "[Subscription]",
            &format!("🧹 已清理上次运行遗留的内核进程（pid {pid}）"),
        );
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
}

/// 控制端口探活（本地 TCP connect，超时 300ms）
fn controller_alive_port(port: u16) -> bool {
    let Ok(address) = format!("127.0.0.1:{port}").parse::<SocketAddr>() else {
        return false;
    };
    std::net::TcpStream::connect_timeout(&address, Duration::from_millis(300)).is_ok()
}

/// 按 pid 杀进程（Windows taskkill / unix kill；进程已退出返回 false）
fn kill_by_pid(pid: u64) -> bool {
    #[cfg(windows)]
    {
        match std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output()
        {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }
    #[cfg(not(windows))]
    {
        match std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output()
        {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }
}

// ─── 端口与监听器分配 ──────────────────────────────────────

/// 控制端口：状态文件里沿用；被别人占了（控制器不认我们的 secret）才换新口。
async fn ensure_controller_port(state: &mut Value, secret: &str) -> Result<u16, String> {
    if let Some(port) = state.get("controllerPort").and_then(json_u16) {
        if controller_alive_port(port) {
            // 有进程在听：是我们的（secret 对上）就沿用（上面已把旧进程杀掉，
            // 端口马上释放）；不是我们的（别人的服务占了）就往下走挑新口
            if controller_get(port, secret, "/version").await.is_some() {
                return Ok(port);
            }
        } else {
            return Ok(port); // 空闲，沿用
        }
    }
    let taken: HashSet<u16> = HashSet::new();
    let port = alloc_port(&taken, CONTROLLER_PORT_BASE, CONTROLLER_PORT_END)
        .ok_or_else(|| format!("没有可用的控制端口（{CONTROLLER_PORT_BASE}-{CONTROLLER_PORT_END} 全被占用）"))?;
    state_insert(state, "controllerPort", json!(port));
    Ok(port)
}

/// 给节点分配监听器端口：已知节点沿用注册表里的端口，新节点从空闲口里挑。
///
/// `sub_name` 从 providers 里按 id 查（provider_nodes 只有 id；节点条目命名
/// 要用订阅名）。
fn allocate_listeners(
    providers: &[ProviderInput],
    ports: &Map<String, Value>,
    provider_nodes: &[(String, Vec<String>)],
) -> Result<Vec<NodeTarget>, String> {
    let mut taken: HashSet<u16> = HashSet::new();
    for port in ports.values().filter_map(json_u16) {
        taken.insert(port);
    }
    taken.insert(CONTROLLER_PORT_BASE);
    let mut targets = Vec::new();
    for (provider_id, nodes) in provider_nodes {
        let sub_name = providers
            .iter()
            .find(|provider| &provider.id == provider_id)
            .map(|provider| provider.name.clone())
            .unwrap_or_default();
        for node in nodes {
            let key = node_key(provider_id, node);
            let port = match ports.get(&key).and_then(json_u16) {
                Some(port) => port,
                None => {
                    let port = alloc_port(&taken, LISTENER_PORT_BASE, LISTENER_PORT_END)
                        .ok_or_else(|| {
                            format!("没有可用的节点端口（{LISTENER_PORT_BASE}-{LISTENER_PORT_END} 用尽）")
                        })?;
                    port
                }
            };
            taken.insert(port);
            targets.push(NodeTarget {
                provider_id: provider_id.clone(),
                sub_name: sub_name.clone(),
                node: node.clone(),
                port,
            });
        }
    }
    Ok(targets)
}

/// 节点集合是否与当前监听器不一致（provider 轮询后判断要不要重建配置）
fn nodes_changed(targets: &[NodeTarget], live: &Map<String, Value>) -> bool {
    for target in targets {
        let Some(names) = live
            .get(&target.provider_id)
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        else {
            continue; // 该 provider 还没拉到（缓存缺失），不当作变化
        };
        if !names.contains(&target.node.as_str()) {
            return true; // 配置里的节点在最新清单里消失了
        }
    }
    for (provider_id, names) in live {
        let Some(names) = names.as_array() else {
            continue;
        };
        for name in names.iter().filter_map(Value::as_str) {
            let known = targets.iter().any(|target| {
                &target.provider_id == provider_id && target.node == name
            });
            if !known {
                return true; // 清单里出现了配置里没有的新节点
            }
        }
    }
    false
}

/// 控制器密钥（随机 32 字节 → hex；getrandom 与 Qoder PKCE 同一依赖）
fn random_secret() -> String {
    let mut bytes = [0u8; 32];
    let _ = getrandom::getrandom(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
