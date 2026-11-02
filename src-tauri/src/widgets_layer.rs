//! 「浮于图标之上」小组件层窗口（widgets-0）：独立顶层透明窗口，位于桌面图标层之上、
//! 普通窗口之下（HWND_BOTTOM）；WM_NCHITTEST 非组件区返回 HTTRANSPARENT 点击穿透
//! （命中判定与共享区域在 utils::hit_test），组件区正常接收鼠标事件。
//! 便签/待办输入时临时摘 WS_EX_NOACTIVATE 抢键盘焦点，WM_KILLFOCUS 自动恢复免激活。
//! Win32 交互密集，本模块不做单测（与 host/attach 同约定），由人工 E2E 覆盖。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use tauri::{AppHandle, Manager};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, ScreenToClient, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, DefWindowProcW, GetWindowLongPtrW, SetForegroundWindow, SetWindowLongPtrW,
    SetWindowPos, GWL_EXSTYLE, GWLP_WNDPROC, HTTRANSPARENT, HWND_BOTTOM, SWP_NOACTIVATE,
    SWP_SHOWWINDOW, WM_KILLFOCUS, WM_NCHITTEST, WNDPROC, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

use crate::desktop_layer::DESKTOP_LAYER_ARGS;
use crate::utils::hit_test;

/// 浮层窗口 label（v1 主显示器；多显示器拓扑属后续任务）
pub const WIDGETS_LABEL: &str = "widgets-0";

/// 已子类化窗口的原窗口过程（key = hwnd；容器窗口激活时也会挂入）。
/// HashMap::new 非 const → OnceLock 惰性初始化
static ORIG_PROCS: OnceLock<Mutex<HashMap<isize, WNDPROC>>> = OnceLock::new();

fn procs() -> &'static Mutex<HashMap<isize, WNDPROC>> {
    ORIG_PROCS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 创建浮层窗口：免激活 + TOOLWINDOW（不进 alt-tab）+ 覆盖主显示器物理矩形 + 置底。
/// 已存在则幂等返回。
pub fn create(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window(WIDGETS_LABEL).is_some() {
        return Ok(());
    }
    let w = tauri::WebviewWindowBuilder::new(
        app,
        WIDGETS_LABEL,
        tauri::WebviewUrl::App("index.html?role=widgets".into()),
    )
    .title("LuminaDesk Widgets")
    .transparent(true)
    .no_redirection_bitmap(true)
    .decorations(false)
    .skip_taskbar(true)
    .resizable(false)
    .visible(false)
    .additional_browser_args(DESKTOP_LAYER_ARGS)
    .build()
    .map_err(|e| e.to_string())?;
    let hwnd = HWND(w.hwnd().map_err(|e| e.to_string())?.0);

    unsafe {
        // 免激活（透明区穿透、组件区点击不抢焦点）+ TOOLWINDOW（不进 alt-tab/taskbar）
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            (ex | WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize,
        );

        // 覆盖主显示器物理矩形；HWND_BOTTOM = 桌面图标之上、普通窗口之下
        let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return Err("GetMonitorInfoW failed".into());
        }
        let rc = info.rcMonitor;
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_BOTTOM),
            rc.left,
            rc.top,
            rc.right - rc.left,
            rc.bottom - rc.top,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }

    install_hit_test(hwnd);
    log::info!("widgets layer created: 0x{:X}", hwnd.0 as isize);
    Ok(())
}

/// 销毁浮层窗口并清理其子类化记录
pub fn destroy(app: &AppHandle) -> Result<(), String> {
    let Some(w) = app.get_webview_window(WIDGETS_LABEL) else {
        return Ok(());
    };
    let hwnd = HWND(w.hwnd().map_err(|e| e.to_string())?.0);
    w.close().map_err(|e| e.to_string())?;
    procs().lock().unwrap().remove(&(hwnd.0 as isize));
    log::info!("widgets layer destroyed");
    Ok(())
}

/// 窗口过程子类化（幂等）：WM_NCHITTEST 穿透 + WM_KILLFOCUS 恢复免激活
fn install_hit_test(hwnd: HWND) {
    let key = hwnd.0 as isize;
    if procs().lock().unwrap().contains_key(&key) {
        return;
    }
    unsafe {
        let prev = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, wnd_proc as *const () as isize);
        let orig: WNDPROC = std::mem::transmute(prev);
        procs().lock().unwrap().insert(key, orig);
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCHITTEST {
        // lparam = 屏幕坐标（有符号分量）→ 转客户区坐标 → 命中判定
        let (sx, sy) = hit_test::unpack_lparam(lparam.0);
        let mut pt = POINT { x: sx, y: sy };
        if unsafe { ScreenToClient(hwnd, &mut pt) }.as_bool() {
            let regions = hit_test::regions_snapshot();
            if !hit_test::hit_at(&regions, pt.x, pt.y) {
                return LRESULT(HTTRANSPARENT as isize); // 非组件区 → 透传给下层
            }
        }
    } else if msg == WM_KILLFOCUS {
        // 输入结束 → 恢复免激活（点击穿透态）；前端 blur 显式调用同样幂等
        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (ex | WS_EX_NOACTIVATE.0) as isize);
        }
    }
    let orig = procs()
        .lock()
        .unwrap()
        .get(&(hwnd.0 as isize))
        .copied()
        .flatten();
    match orig {
        // WNDPROC = Option<fn>（0.61 别名），Some(prev) 即 WNDPROC
        Some(prev) => unsafe { CallWindowProcW(Some(prev), hwnd, msg, wparam, lparam) },
        None => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// 组件输入激活：摘 NOACTIVATE 并抢键盘焦点（便签/待办输入）；
/// 失焦由 WM_KILLFOCUS 自动恢复，前端 blur 也可显式调 false（幂等）。
pub fn set_active(hwnd: HWND, active: bool) {
    install_hit_test(hwnd); // 确保 KILLFOCUS 恢复路径已挂
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let new_ex = if active {
            ex & !WS_EX_NOACTIVATE.0
        } else {
            ex | WS_EX_NOACTIVATE.0
        };
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex as isize);
        if active {
            let _ = SetForegroundWindow(hwnd);
        }
    }
}
