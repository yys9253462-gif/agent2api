use std::sync::OnceLock;
use std::time::Duration;

use tokio::sync::watch;

use crate::port_conflict::StartupFailure;

#[derive(Clone)]
enum StartupState {
    Starting,
    Ready,
    Failed(StartupFailure),
    Stopped,
}

fn state() -> &'static watch::Sender<StartupState> {
    static STATE: OnceLock<watch::Sender<StartupState>> = OnceLock::new();
    STATE.get_or_init(|| watch::channel(StartupState::Starting).0)
}

pub(crate) fn phase() -> &'static str {
    match &*state().borrow() {
        StartupState::Starting => "starting",
        StartupState::Ready => "ready",
        StartupState::Failed(_) => "failed",
        StartupState::Stopped => "stopped",
    }
}

pub(crate) fn complete(result: &Result<(), StartupFailure>) {
    state().send_if_modified(|current| {
        if !matches!(current, StartupState::Starting) {
            return false;
        }
        *current = match result {
            Ok(()) => StartupState::Ready,
            Err(failure) => StartupState::Failed(failure.clone()),
        };
        true
    });
}

pub(crate) fn stop() {
    state().send_replace(StartupState::Stopped);
}

/// 只等待本进程的启动结果，不能把占用同一端口的其它服务当作已就绪。
pub(crate) async fn wait() -> Result<(), String> {
    let mut receiver = state().subscribe();
    let result = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            let current = receiver.borrow_and_update().clone();
            match current {
                StartupState::Ready => return Ok(()),
                StartupState::Failed(failure) => {
                    let message = failure.message.trim();
                    return Err(if message.is_empty() {
                        "本地网关启动失败，请查看运行日志".to_string()
                    } else {
                        format!("本地网关启动失败：{message}")
                    });
                }
                StartupState::Stopped => return Err("本地网关正在退出，无法处理请求".to_string()),
                StartupState::Starting => {}
            }
            receiver.changed().await.map_err(|_| "本地网关启动状态不可用".to_string())?;
        }
    }).await;
    result.unwrap_or_else(|_| Err("本地网关仍在启动，请稍后重试或查看运行日志".to_string()))
}
