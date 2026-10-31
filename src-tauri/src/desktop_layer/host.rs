//! 宿主定位与拓扑探测（spike 实测移植，含修复 3：0x052C 枚举优先、按需发送）。
//!
//! 经典拓扑（Win10 ~ Win11 23H2，本机即此）：
//!   Progman(顶层) ─┬─ WorkerW(壁纸层, 顶层, GetParent()==Progman)  ← 目标宿主
//!                  └─ WorkerW(图标层, 托管 SHELLDLL_DefView)
//! Win11 24H2+（raised desktop）：
//!   Progman 带 WS_EX_NOREDIRECTIONBITMAP，WorkerW 为其直接子窗口，DefView 变 layered 子窗口

use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowExW, FindWindowW, GetClassNameW, GetParent, GetSystemMetrics,
    GetWindowLongPtrW, SendMessageTimeoutW, GWL_EXSTYLE, SM_REMOTESESSION, SMTO_NORMAL,
    WS_EX_NOREDIRECTIONBITMAP,
};

use super::Topology;

/// 强制 Explorer 创建 WorkerW 的未公开消息（社区命名 WM_SHELLPARENTCHANGING）
const MSG_SPAWN_WORKERW: u32 = 0x052C;

pub fn progman_hwnd() -> Option<HWND> {
    // windows 0.61: FindWindowW 可失败 API 返回 Result<HWND>
    unsafe { FindWindowW(w!("Progman"), windows::core::PCWSTR::null()) }.ok()
}

/// 运行时拓扑探测：Progman 的 GWL_EXSTYLE 含 WS_EX_NOREDIRECTIONBITMAP(0x200000) 即为 24H2+ 新拓扑
pub fn detect_topology() -> Topology {
    let Some(progman) = progman_hwnd() else {
        return Topology::None;
    };
    unsafe {
        let ex = GetWindowLongPtrW(progman, GWL_EXSTYLE) as u32;
        if ex & WS_EX_NOREDIRECTIONBITMAP.0 != 0 {
            Topology::Raised24H2
        } else {
            Topology::Classic
        }
    }
}

unsafe extern "system" fn enum_workerw(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
    unsafe {
        let list = &mut *(lparam.0 as *mut Vec<HWND>);
        let mut cls = [0u16; 64];
        let len = GetClassNameW(hwnd, &mut cls);
        if String::from_utf16_lossy(&cls[..len as usize]) == "WorkerW" {
            list.push(hwnd);
        }
    }
    windows::core::BOOL(1)
}

fn find_in_enumeration() -> Option<HWND> {
    let progman = progman_hwnd()?;
    unsafe {
        let mut found: Vec<HWND> = Vec::new();
        let _ = EnumWindows(Some(enum_workerw), LPARAM(&mut found as *mut _ as isize));
        // 实测教训：历次无条件发 0x052C 会不断堆积游离 WorkerW 顶层窗口
        //（spike 积累 19 个），产品化必须「枚举优先，按需发送」。
        // GetParent 返回 NULL 时 Err(1460) 是陈旧 GetLastError 噪音，用 is_ok_and 判断。
        found.into_iter().find(|h| {
            GetParent(*h).is_ok_and(|p| p == progman)
                && FindWindowExW(Some(*h), None, w!("SHELLDLL_DefView"), None).is_err()
        })
    }
}

/// 经典路径：先枚举顶层 WorkerW 找「父窗口为 Progman 且没有 SHELLDLL_DefView 子窗口」者；
/// 找不到才发 0x052C 强制创建（超时 1s 防挂死）后重枚举。
pub fn find_wallpaper_workerw() -> Option<HWND> {
    if let Some(h) = find_in_enumeration() {
        return Some(h);
    }
    let progman = progman_hwnd()?;
    unsafe {
        SendMessageTimeoutW(
            progman,
            MSG_SPAWN_WORKERW,
            WPARAM(0xD),
            LPARAM(0),
            SMTO_NORMAL,
            1000,
            None,
        );
        SendMessageTimeoutW(
            progman,
            MSG_SPAWN_WORKERW,
            WPARAM(0xD),
            LPARAM(1),
            SMTO_NORMAL,
            1000,
            None,
        );
    }
    find_in_enumeration()
}

/// 24H2+ 路径：WorkerW 是 Progman 的直接子窗口（本机不可实测，阶段 2 VM 验证）
pub fn find_raised_workerw() -> Option<HWND> {
    let progman = progman_hwnd()?;
    unsafe { FindWindowExW(Some(progman), None, w!("WorkerW"), None) }.ok()
}

/// RDP 会话检测：RDP 会话无 Progman，桌面层应自动暂停
pub fn is_rdp_session() -> bool {
    unsafe { GetSystemMetrics(SM_REMOTESESSION) != 0 }
}
