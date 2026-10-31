//! 挂载看门狗（spike 实测移植）：
//! 5s 句柄校验 + 60s/5 次限流 + 2s 冷却 + 事件驱动重挂 + 死窗口重建（唯一 label）。
//!
//! 修复 4（S5 实测）：explorer 死后 Tauri 管理器对「外部销毁」的窗口无法释放 label，
//! close() 对已死窗口无效 → 每次重建用新 label（container-{monitor}-g{n}），
//! build 失败记录错误并带 1s 冷却重试 3 次。

use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::UI::WindowsAndMessaging::IsWindow;

use crate::events::REATTACH_LIMIT;
use crate::shell_window::ShellEvent;

use super::{
    attach::{diagnose_attachment, from_isize, is_attached},
    attach_container, container_hwnd, container_label, recreate_container, HOST_HWND,
};

/// 看门狗主循环（阻塞；放专用线程）
pub fn run(app: AppHandle, rx: Receiver<ShellEvent>) {
    let mut attempts: Vec<Instant> = Vec::new();
    loop {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(ev) => {
                log::info!("shell event: {ev:?}");
                reattach(&app, &mut attempts);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // 周期校验缓存句柄（微秒级开销）
                let host = HOST_HWND.load(std::sync::atomic::Ordering::SeqCst);
                if host == 0 {
                    // 从未成功挂载过：直接重试挂载（首挂失败也能自愈）
                    if let Err(e) = attach_container(&app, "watchdog-retry") {
                        log::warn!("watchdog attach retry failed: {e}");
                    } else if let Some(w) = app.get_webview_window(&container_label()) {
                        let _ = w.show();
                    }
                    continue;
                }
                let ok = container_hwnd(&app)
                    .map(|c| is_attached(c, from_isize(host)))
                    .unwrap_or(false);
                if !ok {
                    let diag = container_hwnd(&app)
                        .map(|c| diagnose_attachment(c, from_isize(host)))
                        .unwrap_or_else(|| "container not found".into());
                    log::warn!("watchdog: attachment broken ({diag}), re-attaching");
                    reattach(&app, &mut attempts);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

/// 重挂（60s 内最多 5 次——25H2 反复重挂会永久失效，需限制频率）
pub fn reattach(app: &AppHandle, attempts: &mut Vec<Instant>) {
    let now = Instant::now();
    attempts.retain(|t| now.duration_since(*t) < Duration::from_secs(60));
    if attempts.len() >= 5 {
        log::warn!("reattach limit reached (5/min), giving up");
        let _ = app.emit(REATTACH_LIMIT, ());
        return;
    }
    attempts.push(now);

    // 宿主销毁会连带销毁子窗口：容器 HWND 可能已死，需重建窗口
    let dead = container_hwnd(app)
        .map(|h| !unsafe { IsWindow(Some(h)).as_bool() })
        .unwrap_or(true);
    if dead {
        log::info!("container HWND destroyed — recreating window");
        let _ = recreate_container(app);
    }

    std::thread::sleep(Duration::from_secs(2)); // 冷却，等 explorer 重建桌面
    match attach_container(app, "reattach") {
        Ok(s) => {
            log::info!("reattach: {s}");
            if let Some(w) = app.get_webview_window(&container_label()) {
                let _ = w.show();
            }
        }
        Err(e) => log::warn!("reattach failed: {e}"),
    }
}

/// 宿主重定位（S5/S7/S8 路径）后由事件驱动调用
pub fn trigger_from_event(app: &AppHandle) {
    let mut attempts: Vec<Instant> = Vec::new();
    reattach(app, &mut attempts);
}
