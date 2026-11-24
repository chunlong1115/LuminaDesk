//! 顶层/桌面层窗口共用的原生装配：无边框样式换算与窗口过程子类化。
//!
//! 浮层（widgets-0）与容器（container-*）都经 tauri `decorations(false)` 建窗。
//! tao 对 undecorated 顶层窗口默认开启 decoration_shadow（PlatformSpecific 默认
//! true，tauri 未暴露关闭入口），其 WM_NCCALCSIZE 会按 calculate_insets_for_dpi
//! 把 client 内缩 (8,1,8,8)（96dpi 实测）——webview 偏移且右/下缘裁边。
//! 两窗口都必须让 WM_NCCALCSIZE 返回 0，故把「原过程登记表 + 转发出口」集中在此，
//! 各窗口只保留自己的附加消息分支（浮层：命中穿透/置底守卫；容器：无）。
//! Win32 交互路径不做单测（与 host/attach 同约定），纯函数有单测。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, DefWindowProcW, SetWindowLongPtrW, GWLP_WNDPROC, WS_CAPTION, WS_MAXIMIZEBOX,
    WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME, WNDPROC,
};

/// 已子类化窗口的原窗口过程（key = hwnd）。
/// HashMap::new 非 const → OnceLock 惰性初始化
static ORIG_PROCS: OnceLock<Mutex<HashMap<isize, WNDPROC>>> = OnceLock::new();

fn procs() -> &'static Mutex<HashMap<isize, WNDPROC>> {
    ORIG_PROCS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 无边框样式换算：剥掉标题栏/调整边框类样式位（其余位如 WS_VISIBLE/CLIP* 保留）。
/// WM_NCCALCSIZE 返回 0 后这些位不参与布局，剥除是为了免 Snap Layouts 悬浮、
/// 系统菜单等系统行为。纯函数便于单测
pub(crate) fn borderless_style(style: u32) -> u32 {
    style
        & !(WS_CAPTION.0 | WS_THICKFRAME.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0)
}

/// 安装窗口过程子类化（幂等）：先记录原过程，再挂新过程。
/// 注意：必须在任何会触发 WM_NCCALCSIZE 的重排（SetWindowPos/SetParent/显示）
/// 之前调用，否则那次重排仍由 tao 以阴影内缩作答，wry 会按内缩后的 client
/// 摆放 webview 子窗
pub(crate) fn install_subclass(
    hwnd: HWND,
    proc: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
) {
    let key = hwnd.0 as isize;
    if procs().lock().unwrap().contains_key(&key) {
        return;
    }
    unsafe {
        let prev = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let orig: WNDPROC = std::mem::transmute(prev);
        procs().lock().unwrap().insert(key, orig);
    }
}

/// 清理子类化记录（窗口销毁时调用；链表本身随窗口失效由系统回收）
pub(crate) fn remove_subclass(hwnd: HWND) {
    procs().lock().unwrap().remove(&(hwnd.0 as isize));
}

/// 子类化过程统一出口：转发给原窗口过程（无登记时 DefWindowProc 兜底）
pub(crate) fn call_original(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
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

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::UI::WindowsAndMessaging::{
        WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_VISIBLE,
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
}
