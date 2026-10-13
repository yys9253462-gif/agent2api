//! 内核下载：GitHub Release 资产 → 流式落盘（进度进快照）→ sha256 校验 →
//! 解压安装到内核目录。
//!
//! ── 为什么出口候选与软件更新同一套 ────────────────────────────
//! 下载源是 GitHub（国内直连常常不通），出口顺序与 `update::client` 的
//! `resolveEgressCandidates` 一致：设置的更新出口（`updateProxy` 配置键）
//! 优先、直连次之、Clash 混合端口兜底。`update::client` 是私有模块，这里
//! 复刻同一条顺序（判据一致：只有网络层失败才换出口，HTTP 状态码直接报）。
//!
//! ── 校验失败的整体拒绝 ──────────────────────────────────────
//! sha256 与 pin 死的值（mod.rs 的资产表）对不上就删掉坏文件并报错 ——
//! 代理内核是要接管出网流量的二进制，装一个被篡改的包比装不上严重得多，
//! 不做「校验失败但继续」的降级。
//!
//! ── 解压 ────────────────────────────────────────────────────
//! Windows 资产是 zip（官方只出 zip），解压用系统 PowerShell 的
//! `Expand-Archive`（不引第三方 zip 依赖树；PowerShell 5.1 起 Windows 自带）；
//! macOS / Linux 资产是 .gz，用已在依赖树里的 flate2。解出的二进制统一改名
//! 到 `kernel_binary_name()`（config / 状态文件引用固定名，不随版本漂移）。

use std::io::{Read, Write};
use std::time::Duration;

use serde_json::json;

use crate::server::config;
use crate::server::core::egress;
use crate::server::core::proxies::{resolve_account_proxy, ProxyResolution, CLASH_MIXED_UID};
use crate::server::core::sub_kernel::{kernel_binary_name, kernel_dir, log_line, KERNEL_VERSION};

/// 下载请求的 UA（GitHub 不校验 UA，给个非空的便于排障）
const USER_AGENT: &str = "agent2api";

/// 下载中的进度回报间隔（字节）
const PROGRESS_STEP: u64 = 256 * 1024;

/// 执行下载（worker 调用；已安装时是 no-op ——「重复点下载」不重复拉）。
pub async fn run_download() -> Result<(), String> {
    if crate::server::core::sub_kernel::installed() {
        return Ok(());
    }
    let Some((asset, sha256)) = crate::server::core::sub_kernel::asset_for_current_platform() else {
        return Err("当前平台暂不支持代理内核（官方没有这一平台的预编译产物）".to_string());
    };
    let dir = kernel_dir();
    std::fs::create_dir_all(&dir).map_err(|error| format!("创建内核目录失败: {error}"))?;
    let archive_path = dir.join(asset);
    let url = format!("https://github.com/MetaCubeX/mihomo/releases/download/{KERNEL_VERSION}/{asset}");
    log_line(&format!("⬇️ 开始下载代理内核 {KERNEL_VERSION}（{asset}）"));

    // 依次试各出口（网络层失败才换；HTTP 状态码直接报）
    let mut last_error = String::new();
    for (index, proxy) in egress_candidates().into_iter().enumerate() {
        match download_to_file(&url, &archive_path, proxy.as_ref()).await {
            Ok(()) => {
                last_error.clear();
                break;
            }
            Err(error) => {
                last_error = error;
                if index + 1 < candidate_count() {
                    log_line(&format!("⚠️ {last_error}，换下一条线路重试"));
                }
            }
        }
    }
    if !last_error.is_empty() {
        let _ = std::fs::remove_file(&archive_path);
        return Err(last_error);
    }

    // sha256 校验（对不上整体拒绝，坏文件删掉）
    let actual = file_sha256(&archive_path)?;
    if !actual.eq_ignore_ascii_case(sha256) {
        let _ = std::fs::remove_file(&archive_path);
        return Err("内核下载校验失败（sha256 与官方发布值不匹配），已删除坏文件".to_string());
    }

    // 解压安装（统一改名到固定名）
    install_binary(&archive_path, &dir)?;
    let _ = std::fs::remove_file(&archive_path);
    Ok(())
}

/// 候选出口数（重试日志里「还有没有下一条」的判据）
fn candidate_count() -> usize {
    egress_candidates().len()
}

/// 出口候选：设置的更新出口优先，直连次之，Clash 混合端口兜底
/// （与 update::client 的 resolveEgressCandidates 同一条顺序）。
fn egress_candidates() -> Vec<Option<crate::server::core::proxies::ResolvedProxy>> {
    if let Some(configured) = config::update_proxy() {
        // 指定了更新出口：只用它（「选定出口」的语义就是「走这条线路」）
        if let Some(ProxyResolution::Resolved(proxy)) = resolve_account_proxy(Some(&configured)) {
            if proxy.port.is_some() {
                return vec![Some(proxy)];
            }
        }
    }
    let mut candidates = vec![None];
    if let Some(ProxyResolution::Resolved(proxy)) = resolve_account_proxy(Some(&json!({
        "source": "clash",
        "listenerUid": CLASH_MIXED_UID,
    }))) {
        if proxy.port.is_some() {
            candidates.push(Some(proxy));
        }
    }
    candidates
}

/// 流式下载到文件，进度进快照（界面轮询渲染）。
///
/// 不给总超时（几十 MB 的资产不可能在固定时限内下完）；卡死场景由
/// egress 客户端的 read_timeout 兜底（与软件更新的下载同语义）。
async fn download_to_file(
    url: &str,
    path: &std::path::Path,
    proxy: Option<&crate::server::core::proxies::ResolvedProxy>,
) -> Result<(), String> {
    let client = egress::client_for(proxy);
    let response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|error| format!("无法连接 GitHub: {}", egress::describe_error_detail(&error)))?;
    if !response.status().is_success() {
        return Err(format!("GitHub 返回 {}", response.status().as_u16()));
    }
    let total = response.content_length().unwrap_or(0);
    crate::server::core::sub_kernel::record_download_progress(0, total);
    let mut file = std::fs::File::create(path).map_err(|error| format!("创建下载文件失败: {error}"))?;
    let mut stream = response.bytes_stream();
    use futures::StreamExt;
    let mut received: u64 = 0;
    let mut next_report = PROGRESS_STEP;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("下载中断: {error}"))?;
        file.write_all(&chunk).map_err(|error| format!("写入下载文件失败: {error}"))?;
        received += chunk.len() as u64;
        if received >= next_report {
            next_report = received + PROGRESS_STEP;
            crate::server::core::sub_kernel::record_download_progress(received, total);
        }
    }
    file.flush().map_err(|error| format!("写入下载文件失败: {error}"))?;
    crate::server::core::sub_kernel::record_download_progress(received, total);
    Ok(())
}

/// 文件的 sha256（流式读，不动内存里的整份文件）
fn file_sha256(path: &std::path::Path) -> Result<String, String> {
    use sha2::Digest;
    let mut file =
        std::fs::File::open(path).map_err(|error| format!("打开下载文件失败: {error}"))?;
    let mut hasher = sha2::Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("读取下载文件失败: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// 解压安装：Windows zip 走 PowerShell Expand-Archive，unix .gz 走 flate2；
/// 解出的二进制统一改名到 `kernel_binary_name()`。
fn install_binary(archive_path: &std::path::Path, dir: &std::path::Path) -> Result<(), String> {
    let binary_path = dir.join(kernel_binary_name());
    // 旧二进制先删（Windows 上 rename 到已存在的目标会失败）
    let _ = std::fs::remove_file(&binary_path);
    #[cfg(windows)]
    {
        let extract_dir = dir.join("extract");
        let _ = std::fs::remove_dir_all(&extract_dir);
        std::fs::create_dir_all(&extract_dir)
            .map_err(|error| format!("创建解压目录失败: {error}"))?;
        let archive = archive_path.to_string_lossy().to_string();
        let destination = extract_dir.to_string_lossy().to_string();
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("Expand-Archive -LiteralPath '{archive}' -DestinationPath '{destination}' -Force"),
            ])
            .output()
            .map_err(|error| format!("调用 PowerShell 解压失败: {error}"))?;
        if !output.status.success() {
            let _ = std::fs::remove_dir_all(&extract_dir);
            return Err(format!(
                "解压内核失败（{}）",
                String::from_utf8_lossy(&output.stderr).trim().chars().take(200).collect::<String>()
            ));
        }
        // 解出的 exe 名是 `mihomo-windows-amd64-<版本>.exe`，改名到固定名
        let extracted = std::fs::read_dir(&extract_dir)
            .map_err(|error| format!("读取解压目录失败: {error}"))?
            .filter_map(|entry| entry.ok())
            .find(|entry| {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                name.starts_with("mihomo-") && name.ends_with(".exe")
            })
            .ok_or_else(|| "解压结果里没有内核二进制".to_string())?;
        std::fs::rename(extracted.path(), &binary_path)
            .map_err(|error| format!("安装内核失败: {error}"))?;
        let _ = std::fs::remove_dir_all(&extract_dir);
    }
    #[cfg(not(windows))]
    {
        let archive = std::fs::File::open(archive_path)
            .map_err(|error| format!("打开下载文件失败: {error}"))?;
        let mut decoder = flate2::read::GzDecoder::new(archive);
        let mut binary = std::fs::File::create(&binary_path)
            .map_err(|error| format!("安装内核失败: {error}"))?;
        std::io::copy(&mut decoder, &mut binary)
            .map_err(|error| format!("解压内核失败: {error}"))?;
        // 可执行位（unix 上从网络下载的二进制默认没有执行权限）
        use std::os::unix::fs::PermissionsExt;
        let _ = binary.set_permissions(std::fs::Permissions::from_mode(0o755));
    }
    Ok(())
}

/// 等待一段时间的辅助（下载重试之间用）
#[allow(dead_code)]
fn sleep_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}
