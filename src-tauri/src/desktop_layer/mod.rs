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
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_STYLE, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOZORDER, WM_NCCALCSIZE,
};

use crate::events::LAYER_CHANGED;
use crate::utils::dpi::physical_to_logical;
use crate::window_assembly::{borderless_style, call_original, install_subclass};

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

/// 容器默认物理几何：主显示器原点 + 偏移，560×460（v1 固定值；
/// 布局持久化接入后改读存储）。建窗与 attach 必须共用同一组常量
pub const CONTAINER_OFFSET: (i32, i32) = (40, 40);
pub const CONTAINER_SIZE: (i32, i32) = (560, 460);

/// 创建容器窗口（visible:false 创建、挂载后 Show，避免闪烁）。
/// 建窗即按目标屏幕逻辑几何落地（position+inner_size，同浮层）：visible(false)+
/// 默认尺寸会让 WebView2 按初始上下文冻结缩放因子（125% 缩放下实测 dpr 恒 1.0）。
/// 建窗后立即子类化并把 WM_NCCALCSIZE 归零，随后的显式 SetWindowPos 才能让
/// client=outer、wry 把 webview 子窗排满整窗（修掉 tao shadow 内缩 (8,1,8,8)）。
pub fn create_container_window(app: &AppHandle, label: &str) -> Option<WebviewWindow> {
    // debug 构建携带诊断参数（S4 滴答验证）；release 不带（零开销）
    let url = if cfg!(debug_assertions) {
        "index.html?role=container&diag=1"
    } else {
        "index.html?role=container"
    };
    // 主显示器物理原点与缩放因子（与 attach 的 MonitorFromPoint(0,0) 同一基准）
    let monitor = app.monitor_from_point(0.0, 0.0).ok().flatten();
    let scale = monitor.as_ref().map(|m| m.scale_factor()).unwrap_or(1.0);
    let (mx, my) = monitor
        .as_ref()
        .map(|m| (m.position().x, m.position().y))
        .unwrap_or((0, 0));
    // tauri 建窗参数以逻辑像素计，Monitor::position/size 为物理像素
    let (lx, ly) = physical_to_logical((mx + CONTAINER_OFFSET.0, my + CONTAINER_OFFSET.1), scale);
    let (lw, lh) = physical_to_logical(CONTAINER_SIZE, scale);
    log::info!(
        "container window geometry: monitor physical=({mx},{my}) scale={scale} -> logical=({lx},{ly}), size=({lw},{lh})"
    );

    let w = tauri::WebviewWindowBuilder::new(
        app,
        label,
        tauri::WebviewUrl::App(url.into()),
    )
    .title("LuminaDesk Container")
    .position(lx, ly)
    .inner_size(lw, lh)
    .transparent(true)
    .no_redirection_bitmap(true)
    .decorations(false)
    .skip_taskbar(true)
    .resizable(false)
    .visible(false)
    .additional_browser_args(DESKTOP_LAYER_ARGS)
    .build()
    .map_err(|e| log::error!("container window build failed: {e}"))
    .ok()?;
    let Ok(h) = w.hwnd() else {
        log::error!("container window hwnd unavailable");
        return Some(w);
    };
    let hwnd = HWND(h.0);

    // 子类化必须先于任何重排（同浮层）：NCCALCSIZE 归零后 client=outer，
    // 后续 SetWindowPos/attach 才能让 wry 按整窗摆放 webview
    install_subclass(hwnd, container_wnd_proc);
    unsafe {
        // 剥标题栏/调整边框类样式位（decorations(false) 之外的兜底；
        // 该位会引出 Snap Layouts/系统菜单等系统行为，对桌面层无意义）
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        SetWindowLongPtrW(hwnd, GWL_STYLE, borderless_style(style) as isize);
        // 显式落地物理矩形（保持隐藏）：校准创建期 tao shadow 造成的 client 内缩，
        // SWP_FRAMECHANGED 触发 WM_NCCALCSIZE（返回 0）与 WM_SIZE → wry 重排子窗
        let _ = SetWindowPos(
            hwnd,
            None,
            mx + CONTAINER_OFFSET.0,
            my + CONTAINER_OFFSET.1,
            CONTAINER_SIZE.0,
            CONTAINER_SIZE.1,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
    Some(w)
}

/// 容器窗口过程：WM_NCCALCSIZE 归零（client=outer，修掉 tao shadow 内缩），
/// 其余消息交还 tao。容器不需要命中穿透/z 序守卫（点击与 z 序由宿主 WorkerW 管理）
unsafe extern "system" fn container_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCALCSIZE {
        return LRESULT(0);
    }
    call_original(hwnd, msg, wparam, lparam)
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
    // v1：与建窗共用同一默认物理几何（布局持久化接入后改读存储）
    attach::attach(
        container,
        host,
        topo,
        CONTAINER_OFFSET.0,
        CONTAINER_OFFSET.1,
        CONTAINER_SIZE.0,
        CONTAINER_SIZE.1,
    )?;
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
