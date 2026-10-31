//! 桌面层服务入口：状态、窗口创建/重建、挂载/卸载、状态查询。
//!
//! spike 已验证的四大关键实现（<spike-dir> 实测，本模块为产品化移植）：
//! 1. 启动顺序：先 show → 等 1.5s tao 样式落定 → 再 attach（否则 GWL_STYLE 重应用断父链）
//! 2. 透明：attach 时加 WS_EX_NOREDIRECTIONBITMAP（tao blur-behind 对子窗口失效，黑块根因）
//! 3. find_wallpaper_workerw 枚举优先（父==Progman 且无 SHELLDLL_DefView），找不到才发 0x052C
//! 4. 重建唯一 label：explorer 死后 Tauri label 永久泄漏，重建用 container-{monitor}-g{n}

pub mod attach;
pub mod geometry;
pub mod host;
pub mod watchdog;

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use windows::Win32::Foundation::HWND;

use crate::events::LAYER_CHANGED;

pub use attach::{from_isize, to_isize};

/// 运行时桌面拓扑
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topology {
    /// Win10 ~ Win11 23H2：壁纸层是顶层 WorkerW，GetParent() == Progman
    Classic,
    /// Win11 24H2+：WorkerW 是 Progman 的直接子窗口（本机不可实测，阶段 2 VM 验证）
    Raised24H2,
    /// 无 Progman（RDP 等会话）
    None,
}

/// 当前挂载宿主（WorkerW/Progman）句柄；0 = 未挂载
pub static HOST_HWND: AtomicIsize = AtomicIsize::new(0);
pub static ATTACHED: AtomicBool = AtomicBool::new(false);
/// 重建代数：0 = 初代窗口（label `container-{monitor}`）
static RECREATE_COUNT: AtomicU32 = AtomicU32::new(0);
/// 当前容器窗口 label（重建后更新为 `container-{monitor}-g{n}`）
static CONTAINER_LABEL: Mutex<String> = Mutex::new(String::new());

pub fn container_label() -> String {
    let l = CONTAINER_LABEL.lock().unwrap().clone();
    if l.is_empty() {
        "container-0".into()
    } else {
        l
    }
}

pub fn container_hwnd(app: &AppHandle) -> Option<HWND> {
    let w = app.get_webview_window(&container_label())?;
    match w.hwnd() {
        Ok(h) => Some(HWND(h.0)),
        Err(e) => {
            log::warn!("hwnd() error: {e}");
            None
        }
    }
}

/// 桌面层窗口防节流 flags 四件套（S4 实测：被遮挡 60s 滴答不间断）。
/// 注意：同一应用内所有窗口的 additionalBrowserArgs 必须完全一致
///（Tauri 2 单一共享 WebView2 环境按首个窗口参数初始化——spike 0x8007139F 根因）。
pub const DESKTOP_LAYER_ARGS: &str = "--disable-background-timer-throttling --disable-backgrounding-occluded-windows --disable-renderer-backgrounding --disable-features=CalculateNativeWinOcclusion";

/// 创建容器窗口（visible:false 创建、挂载后 Show，避免闪烁）
pub fn create_container_window(app: &AppHandle, label: &str) -> Option<WebviewWindow> {
    // debug 构建携带诊断参数（S4 滴答验证）；release 不带（零开销）
    let url = if cfg!(debug_assertions) {
        "index.html?role=container&diag=1"
    } else {
        "index.html?role=container"
    };
    tauri::WebviewWindowBuilder::new(
        app,
        label,
        tauri::WebviewUrl::App(url.into()),
    )
    .title("LuminaDesk Container")
    .inner_size(560.0, 460.0)
    .transparent(true)
    .no_redirection_bitmap(true)
    .decorations(false)
    .skip_taskbar(true)
    .resizable(false)
    .visible(false)
    .additional_browser_args(DESKTOP_LAYER_ARGS)
    .build()
    .map_err(|e| log::error!("container window build failed: {e}"))
    .ok()
}

/// 重建容器窗口（修复 4：不调用 close()——对已死窗口无效且 label 泄漏；
/// 用新 label + 冷却重试，最多 3 次，每次 1s）
pub fn recreate_container(app: &AppHandle) -> Option<WebviewWindow> {
    let monitor = 0u32;
    let label = format!(
        "container-{monitor}-g{}",
        RECREATE_COUNT.fetch_add(1, Ordering::SeqCst) + 1
    );
    for attempt in 0..3 {
        match create_container_window(app, &label) {
            Some(w) => {
                *CONTAINER_LABEL.lock().unwrap() = label;
                return Some(w);
            }
            None => {
                log::warn!("recreate attempt {attempt} failed, retrying in 1s");
                std::thread::sleep(Duration::from_millis(1000));
            }
        }
    }
    log::error!("recreate: all attempts failed");
    None
}

/// 完整挂载流程：拓扑探测 → 宿主定位 → SetParent → 状态更新 + 事件广播
pub fn attach_container(app: &AppHandle, why: &str) -> Result<String, String> {
    if host::is_rdp_session() {
        return Err("RDP session — desktop layer paused".into());
    }
    let topo = host::detect_topology();
    if topo == Topology::None {
        return Err("no Progman (RDP or shell not ready)".into());
    }
    let host = match topo {
        Topology::Classic => host::find_wallpaper_workerw(),
        Topology::Raised24H2 => host::find_raised_workerw(),
        Topology::None => unreachable!(),
    }
    .ok_or("host window not found")?;
    let container = container_hwnd(app).ok_or("container window not found")?;
    // v1：默认位置 (40,40) 物理像素（布局持久化接入后改读存储）
    attach::attach(container, host, topo, 40, 40, 560, 460)?;
    HOST_HWND.store(to_isize(host), Ordering::SeqCst);
    ATTACHED.store(true, Ordering::SeqCst);
    log::info!("attach ok ({why}) topo={topo:?} host=0x{:X}", to_isize(host));
    let _ = app.emit(
        LAYER_CHANGED,
        serde_json::json!({ "attached": true, "topology": format!("{topo:?}") }),
    );
    Ok("attached".into())
}

pub fn detach_container(app: &AppHandle) -> Result<String, String> {
    if let Some(c) = container_hwnd(app) {
        attach::detach(c);
    }
    ATTACHED.store(false, Ordering::SeqCst);
    HOST_HWND.store(0, Ordering::SeqCst);
    log::info!("detached");
    let _ = app.emit(LAYER_CHANGED, serde_json::json!({ "attached": false }));
    Ok("detached".into())
}

/// 状态查询（前端「刷新状态」与诊断用）
pub fn layer_status(app: &AppHandle) -> String {
    let topo = host::detect_topology();
    let host = HOST_HWND.load(Ordering::SeqCst);
    let container = container_hwnd(app);
    let attached_flag = ATTACHED.load(Ordering::SeqCst);
    let verified = match (container, host) {
        (Some(c), h) if h != 0 => attach::is_attached(c, from_isize(h)),
        _ => false,
    };
    format!(
        "topology: {topo:?}\nhost(WorkerW): 0x{host:X}\ncontainer: {}\nattached(flag): {attached_flag}\nverified: {verified}\nrdp_session: {}",
        container
            .map(|h| format!("0x{:X}", to_isize(h)))
            .unwrap_or_else(|| "not-found".into()),
        host::is_rdp_session(),
    )
}
