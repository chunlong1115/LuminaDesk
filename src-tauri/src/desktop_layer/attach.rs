//! SetParent 执行（spike 实测移植，含修复 1/2/4 的相关约束）。
//!
//! 修复 2（透明）：tao 顶层透明法（blur-behind）在 SetParent 成子窗口后失效，
//! GDI 未绘制的重定向缓冲露出黑底（S2 实测黑块根因）；加 WS_EX_NOREDIRECTIONBITMAP
//! 去掉 GDI 后台缓冲后，窗口表面只剩 WebView2 内容参与合成，透明区自然透出壁纸。

use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetParent, GetWindowLongPtrW, GetWindowRect, IsWindow, SetParent, SetWindowLongPtrW,
    SetWindowPos, GWL_EXSTYLE, GWL_STYLE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOZORDER,
    SWP_SHOWWINDOW, WS_CHILD, WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP, WS_POPUP,
};

use super::Topology;

pub fn to_isize(h: HWND) -> isize {
    h.0 as isize
}

pub fn from_isize(v: isize) -> HWND {
    HWND(v as *mut core::ffi::c_void)
}

/// 挂载：加 NOACTIVATE+NRB → WS_POPUP 换 WS_CHILD → SetParent 进宿主 → 定位。
/// 两拓扑共用同一流程：先置 WS_CHILD 才能让 SetParent 建立真正的父子关系
/// （否则 WS_POPUP 窗口 SetParent 只建立 owner 关系，GetParent 会返回 NULL——MSDN 已知坑）。
/// 24H2 是否还需额外步骤（置于 DefView 之下）本机不可实测，待 VM 验证。
pub fn attach(hwnd: HWND, host: HWND, topology: Topology, x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
    unsafe {
        // 1. 不抢焦点、不进任务栏；加 NRB 实现透明（见模块注释）
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            (ex | WS_EX_NOACTIVATE.0 | WS_EX_NOREDIRECTIONBITMAP.0) as isize,
        );

        // 2. POPUP → CHILD（坐标将相对宿主客户区）
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        SetWindowLongPtrW(hwnd, GWL_STYLE, ((style & !WS_POPUP.0) | WS_CHILD.0) as isize);

        // 3. 挂载 + 双重校验（失败等 500ms 再查一次——偶发异步生效）
        let prev = SetParent(hwnd, Some(host));
        let cur = GetParent(hwnd);
        let ok = cur.as_ref().is_ok_and(|p| *p == host);
        if !ok {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let cur2 = GetParent(hwnd);
            return Err(format!(
                "SetParent 后父窗口未生效: prev={prev:?}, cur={cur:?}, cur(+500ms)={cur2:?}, \
                 hwnd=0x{:X}, host=0x{:X}, topo={topology:?}",
                to_isize(hwnd),
                to_isize(host)
            ));
        }

        // 4. 宿主客户区原点（= 宿主窗口矩形原点）→ 物理坐标换算
        let mut wr = RECT::default();
        if GetWindowRect(host, &mut wr).is_err() {
            return Err("GetWindowRect(host) 失败".into());
        }
        let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTONEAREST);
        if monitor.0.is_null() {
            return Err("MonitorFromPoint 失败".into());
        }
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT::default(),
            rcWork: RECT::default(),
            dwFlags: 0,
        };
        if !GetMonitorInfoW(monitor, &mut mi).as_bool() {
            return Err("GetMonitorInfoW 失败".into());
        }
        let cx = mi.rcMonitor.left - wr.left + x;
        let cy = mi.rcMonitor.top - wr.top + y;
        let _ = SetWindowPos(
            hwnd,
            None,
            cx,
            cy,
            w,
            h,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_FRAMECHANGED,
        );
        Ok(())
    }
}

/// 卸载：SetParent(NULL) 恢复为顶层窗口，并挪回屏幕可见位置（同时还原 WS_POPUP）
pub fn detach(hwnd: HWND) {
    unsafe {
        let _ = SetParent(hwnd, None);
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        SetWindowLongPtrW(hwnd, GWL_STYLE, ((style & !WS_CHILD.0) | WS_POPUP.0) as isize);
        let _ = SetWindowPos(
            hwnd,
            None,
            200,
            200,
            560,
            460,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

pub fn is_attached(hwnd: HWND, host: HWND) -> bool {
    unsafe {
        IsWindow(Some(hwnd)).as_bool()
            && IsWindow(Some(host)).as_bool()
            && GetParent(hwnd).is_ok_and(|p| p == host)
    }
}

/// 诊断附件状态：定位「看门狗误报/真断裂」（tauri 延迟样式应用会把 WS_CHILD 改回 WS_POPUP 等）
pub fn diagnose_attachment(hwnd: HWND, host: HWND) -> String {
    unsafe {
        let child_alive = IsWindow(Some(hwnd)).as_bool();
        let host_alive = IsWindow(Some(host)).as_bool();
        let parent = GetParent(hwnd);
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        format!(
            "child_alive={child_alive} host_alive={host_alive} parent={parent:?} \
             ws_child={} ws_popup={}",
            style & WS_CHILD.0 != 0,
            style & WS_POPUP.0 != 0
        )
    }
}
