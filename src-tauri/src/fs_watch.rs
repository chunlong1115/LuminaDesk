//! 桌面目录文件监听：notify 事件 300ms 去抖 → lumina://fs-changed 广播。
//! 前端收到事件后重新拉取 container_list_items（前端零轮询）。

use std::path::PathBuf;
use std::time::{Duration, Instant};

use notify::{recommended_watcher, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};

use crate::events::FS_CHANGED;

/// 启动监听线程（阻塞；放专用线程）。返回 watcher（须保持存活）。
pub fn spawn(app: AppHandle, dir: PathBuf) -> Result<(), String> {
    std::thread::Builder::new()
        .name("lumina-fs-watch".into())
        .spawn(move || {
            if let Err(e) = watch_loop(app, dir) {
                log::error!("fs watcher exited: {e}");
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn watch_loop(app: AppHandle, dir: PathBuf) -> notify::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher: RecommendedWatcher = recommended_watcher(tx)?;
    watcher.watch(&dir, RecursiveMode::NonRecursive)?;
    log::info!("watching desktop dir: {}", dir.display());

    let mut pending = false;
    loop {
        // 首事件等待；后续事件以 300ms 去抖窗口收集
        let ev = if pending {
            match rx.recv_timeout(Duration::from_millis(300)) {
                Ok(Ok(ev)) => ev,
                Ok(Err(e)) => {
                    log::warn!("fs watch error: {e}");
                    pending = false;
                    continue;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // 窗口内无新事件 → 广播
                    let _ = app.emit(FS_CHANGED, ());
                    pending = false;
                    continue;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match rx.recv() {
                Ok(Ok(ev)) => ev,
                Ok(Err(e)) => {
                    log::warn!("fs watch error: {e}");
                    continue;
                }
                Err(_) => break,
            }
        };
        if is_relevant(&ev) {
            pending = true;
        }
    }
    Ok(())
}

fn is_relevant(ev: &notify::Event) -> bool {
    use notify::EventKind;
    // 仅关心名称/创建/删除类事件；访问/元数据等忽略
    matches!(
        ev.kind,
        EventKind::Create(_)
            | EventKind::Remove(_)
            | EventKind::Modify(notify::event::ModifyKind::Name(_))
            | EventKind::Modify(notify::event::ModifyKind::Any)
    )
}
