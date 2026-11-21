//! 「浮于图标之上」小组件层窗口（widgets-0）：独立顶层透明窗口，位于桌面图标层之上、
//! 普通窗口之下（HWND_BOTTOM）；WM_NCHITTEST 非组件区返回 HTTRANSPARENT 点击穿透
//! （命中判定与共享区域在 utils::hit_test），组件区正常接收鼠标事件。
//! 便签/待办输入时 SetForegroundWindow 抢键盘焦点后立即置底收回桌面带：物理键盘只
//! 路由到前台线程的焦点窗口，必须激活；但 z 序与激活态正交——窗口保持前台/焦点继续
//! 收键，同时待在桌面带，透明区点击穿透到其上方的桌面窗口，输入期间桌面依然可点。
//! 旧实现抬顶后要等失焦才复位，其上方的全屏窗口会吞掉桌面点击且 blur 永不送达
//! （E2E「桌面不可操作」根因）；NOACTIVATE 全程保留（点击不激活不抬窗），
//! WM_KILLFOCUS 防御性置底兜底，WM_WINDOWPOSCHANGING 守卫拦截一切抬窗（双保险）。
//! 建窗即按主显示器逻辑几何落地（position+inner_size），避免 visible(false)+默认尺寸
//! 再原生 SetWindowPos 改几何导致 WebView2 dpr 冻结为 1.0（125% 缩放下实测）。
//! 建窗后再显式 SetWindowPos 到物理屏幕矩形（见 create 内注释）。
//! Win32 交互路径不做单测（与 host/attach 同约定），纯判定函数有单测，其余人工 E2E。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use tauri::{AppHandle, Manager};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, DefWindowProcW, GetWindowLongPtrW, SetForegroundWindow, SetWindowLongPtrW,
    SetWindowPos, WINDOWPOS, GWL_EXSTYLE, GWL_STYLE, GWLP_WNDPROC, HTTRANSPARENT, HWND_BOTTOM,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW,
    WM_KILLFOCUS, WM_NCCALCSIZE, WM_NCHITTEST, WM_WINDOWPOSCHANGING, WNDPROC, WS_CAPTION,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
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

/// 物理像素 → 逻辑像素（tauri 建窗参数 position/inner_size 以逻辑像素计；
/// Monitor::size/position 返回物理像素，需按 scale_factor 折算）
pub(crate) fn physical_to_logical(physical: (i32, i32), scale: f64) -> (f64, f64) {
    (physical.0 as f64 / scale, physical.1 as f64 / scale)
}

/// 无边框样式换算：剥掉标题栏/调整边框类样式位（其余位如 WS_VISIBLE/CLIP* 保留）。
/// WM_NCCALCSIZE 返回 0 后这些位不参与布局，剥除是为了免 Snap Layouts 悬浮、
/// 系统菜单等系统行为。纯函数便于单测
pub(crate) fn borderless_style(style: u32) -> u32 {
    style
        & !(WS_CAPTION.0 | WS_THICKFRAME.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0)
}

/// WM_WINDOWPOSCHANGING 守卫判定：凡未声明 SWP_NOZORDER 且目标不是 HWND_BOTTOM 的
/// z 序调整一律强制置底——窗口必须永远留在桌面带（抬顶会把透明区点击透传引向其上方
/// 的全屏窗口被吞，E2E「桌面不可操作」根因；置底调用本身放行）
pub(crate) fn force_bottom_on_change(flags: u32, insert_after: isize) -> bool {
    flags & SWP_NOZORDER.0 == 0 && insert_after != HWND_BOTTOM.0 as isize
}

/// 创建浮层窗口：免激活 + TOOLWINDOW（不进 alt-tab）+ 覆盖主显示器 + 置底。
/// 已存在则幂等返回。
/// 几何在建窗参数按逻辑像素落地（而非建窗后原生 SetWindowPos 改尺寸）：
/// visible(false)+默认尺寸建窗会让 WebView2 缩放因子按初始上下文冻结，
/// 125% 缩放下实测 dpr 恒 1.0；建窗即给足目标几何则按其所在显示器 DPI 正确初始化。
/// 建窗后剥标题栏/调整边框类样式，并显式 SetWindowPos 到物理屏幕矩形。
/// client 偏移的根因不是样式而是 tao 的 WM_NCCALCSIZE 阴影内缩
/// （见 wnd_proc 的 WM_NCCALCSIZE 分支注释）。
pub fn create(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window(WIDGETS_LABEL).is_some() {
        return Ok(());
    }
    // 主显示器逻辑几何（v1 单显示器；多显示器拓扑属后续任务）
    let monitor = app
        .monitor_from_point(0.0, 0.0)
        .map_err(|e| e.to_string())?
        .ok_or("primary monitor not found")?;
    let scale = monitor.scale_factor();
    let (lx, ly) = physical_to_logical((monitor.position().x, monitor.position().y), scale);
    let (lw, lh) = physical_to_logical(
        (monitor.size().width as i32, monitor.size().height as i32),
        scale,
    );
    log::info!(
        "widgets layer geometry: monitor physical=({},{}) pos=({},{}) scale={} -> logical=({},{}), size=({},{})",
        monitor.size().width,
        monitor.size().height,
        monitor.position().x,
        monitor.position().y,
        scale,
        lx,
        ly,
        lw,
        lh
    );

    let w = tauri::WebviewWindowBuilder::new(
        app,
        WIDGETS_LABEL,
        tauri::WebviewUrl::App("index.html?role=widgets".into()),
    )
    .title("LuminaDesk Widgets")
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
    .map_err(|e| e.to_string())?;
    let hwnd = HWND(w.hwnd().map_err(|e| e.to_string())?.0);

    // 子类化必须先于下方几何 SetWindowPos 挂上：否则那次重排的 WM_NCCALCSIZE 仍由
    // tao 以阴影内缩作答，WM_SIZE 时 wry 会按内缩后的 client 尺寸摆放 webview 子窗
    // （实测子窗 1904×1071 而非满屏）；先挂上则 NCCALCSIZE 返回 0、client=满屏，
    // 随后的 WM_SIZE 让 wry 把子窗排到整窗
    install_hit_test(hwnd);

    unsafe {
        // 免激活（透明区穿透、组件区点击不抢焦点）+ TOOLWINDOW（不进 alt-tab/taskbar）
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            (ex | WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize,
        );

        // 无边框：剥标题栏/调整边框类样式。tao 只对 CHILD 窗口剥 WS_CAPTION；
        // 顶层窗口保留的系统菜单/最大化按钮位会引出 Snap Layouts 悬浮、系统菜单
        // 等系统行为，对桌面层无意义。注意：client 偏移 (8,1) 的根因不是样式，
        // 而是 tao 的 WM_NCCALCSIZE 阴影内缩（见 wnd_proc 的 WM_NCCALCSIZE 分支）
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        SetWindowLongPtrW(hwnd, GWL_STYLE, borderless_style(style) as isize);

        // 置底（桌面图标之上、普通窗口之下）+ 显式落地物理屏幕矩形 + FRAMECHANGED
        // 重排 client + 首次显示。SetWindowPos 以物理像素计（进程 PMv2），直接取
        // monitor 物理值；WM_NCCALCSIZE 返回 0 后 outer=client=屏幕矩形。
        // 若不显式带尺寸，创建期 outer（逻辑尺寸+阴影内缩，实测 1936×1089）会残留
        let (mx, my) = (monitor.position().x, monitor.position().y);
        let (mw, mh) = (monitor.size().width as i32, monitor.size().height as i32);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_BOTTOM),
            mx,
            my,
            mw,
            mh,
            SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_FRAMECHANGED,
        );
    }

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
    if msg == WM_NCCALCSIZE {
        // 整窗即客户区（直接返回 0，不转发 tao）：tao 对 undecorated 顶层窗口
        // 默认开 decoration_shadow（PlatformSpecific 默认 true，tauri 未暴露关闭
        // 入口），其 WM_NCCALCSIZE 会把 client 按 calculate_insets_for_dpi 内缩
        // (8,1,8,8)（96dpi 实测）——webview 偏移 (8,1) 且右/下缘裁边，与
        // 「适应桌面大小」逐像素违背；本子类化先于 tao 执行，返回 0 使
        // client=outer=屏幕矩形。注：本子类化只挂在浮层窗口上
        return LRESULT(0);
    } else if msg == WM_NCHITTEST {
        // lparam = 屏幕坐标（有符号分量）→ 转客户区坐标 → 命中判定
        let (sx, sy) = hit_test::unpack_lparam(lparam.0);
        let mut pt = POINT { x: sx, y: sy };
        if unsafe { ScreenToClient(hwnd, &mut pt) }.as_bool() {
            let regions = hit_test::regions_snapshot();
            if !hit_test::hit_at(&regions, pt.x, pt.y) {
                return LRESULT(HTTRANSPARENT as isize); // 非组件区 → 透传给下层
            }
        }
    } else if msg == WM_WINDOWPOSCHANGING {
        // 守卫：拦截一切非置底的 z 序调整（防第三方/框架 SetWindowPos 抬窗）
        let wp = unsafe { &mut *(lparam.0 as *mut WINDOWPOS) };
        if force_bottom_on_change(wp.flags.0, wp.hwndInsertAfter.0 as isize) {
            wp.hwndInsertAfter = HWND_BOTTOM;
        }
    } else if msg == WM_KILLFOCUS {
        // 输入结束 → 恢复免激活 + 防御性置底（双保险）：set_active(true) 取焦后
        // 已立即置底，此处兜底防任何外因抬窗后霸顶——窗口霸顶时透明区点击
        // 透传会被其上方的全屏窗口吞掉，桌面不可操作；前端 blur 显式调
        // set_active(false) 同样幂等
        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (ex | WS_EX_NOACTIVATE.0) as isize);
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_BOTTOM),
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
            );
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

/// 组件输入激活：SetForegroundWindow 抢键盘焦点后立即置底收回桌面带
/// （便签/待办输入）。物理键盘只路由到前台线程的焦点窗口，必须激活；
/// 置底（SWP_NOACTIVATE）不改变激活态——窗口保持前台/焦点继续收键，
/// 同时待在桌面带，其上方的桌面窗口照常接收透明区透传点击。
/// 点击触发本函数时 last-input 授权保证 SetForegroundWindow 成功；
/// 失焦由 WM_KILLFOCUS 兜底（恢复免激活 + 置底），前端 blur 显式调
/// false 幂等。
pub fn set_active(hwnd: HWND, active: bool) {
    install_hit_test(hwnd); // 确保 KILLFOCUS 兜底路径已挂
    unsafe {
        if active {
            // 激活 → 前台 + 焦点（WM_SETFOCUS → wry 子类化 MoveFocus →
            // WebView2 接管键盘）；同时抬顶，立即收回
            let _ = SetForegroundWindow(hwnd);
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_BOTTOM),
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
            );
        } else {
            // 输入结束防御性置底：把任何意外抬窗收回桌面带
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_BOTTOM),
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::UI::WindowsAndMessaging::{
        SWP_NOMOVE, SWP_NOSIZE, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_VISIBLE,
    };

    #[test]
    fn borderless_style_strips_frame_bits() {
        // 标题栏/调整边框/系统菜单/最小化/最大化位全部剥除
        let frame_bits = WS_CAPTION.0
            | WS_THICKFRAME.0
            | WS_SYSMENU.0
            | WS_MINIMIZEBOX.0
            | WS_MAXIMIZEBOX.0;
        assert_eq!(borderless_style(frame_bits), 0);
        // 其余位（VISIBLE/CLIPSIBLINGS/CLIPCHILDREN）原样保留
        let kept = WS_VISIBLE.0 | WS_CLIPSIBLINGS.0 | WS_CLIPCHILDREN.0;
        assert_eq!(borderless_style(frame_bits | kept), kept);
        // 空样式幂等
        assert_eq!(borderless_style(0), 0);
    }

    #[test]
    fn physical_to_logical_scales() {
        // 125%：1920×1080 物理 → 1536×864 逻辑
        assert_eq!(physical_to_logical((1920, 1080), 1.25), (1536.0, 864.0));
        // 100%：原样
        assert_eq!(physical_to_logical((1920, 1080), 1.0), (1920.0, 1080.0));
        // 负坐标（副屏位置）按比例折算
        assert_eq!(physical_to_logical((-320, 40), 2.0), (-160.0, 20.0));
    }

    #[test]
    fn force_bottom_decisions() {
        // 未声明 NOZORDER 且目标非置底 → 强制置底（无例外：窗口必须永远留在桌面带）
        assert!(force_bottom_on_change(0, 2));
        assert!(force_bottom_on_change(SWP_NOMOVE.0, 2));
        // 声明 NOZORDER 的调整放行（只改位置/尺寸不动 z 序）
        assert!(!force_bottom_on_change(SWP_NOZORDER.0, 2));
        // 目标是 HWND_BOTTOM 的调整放行
        assert!(!force_bottom_on_change(0, HWND_BOTTOM.0 as isize));
        assert!(!force_bottom_on_change(SWP_NOSIZE.0, HWND_BOTTOM.0 as isize));
    }
}
