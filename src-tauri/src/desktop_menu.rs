//! 桌面右键替换（可选功能）：WH_MOUSE_LL 低级鼠标钩子拦截桌面区域（Progman/
//! WorkerW/SHELLDLL_DefView）右键，转发给容器前端显示自定义菜单，并吞掉系统默认菜单。
//!
//! 设计约束：
//! - 钩子挂在专用线程（LL 钩子要求安装线程有消息泵），channel 控制安装/卸载
//! - 命中过滤逻辑抽纯函数（class_is_desktop 可单测）
//! - 开关持久化到 settings.json（persistence::models::Settings.desktop_menu_enabled）
//! - 风险标注：全局钩子对安全软件可见；阶段 5 签名后复验

use std::sync::mpsc::{Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetClassNameW, GetMessageW, GetParent, GetWindowRect, SetWindowsHookExW,
    UnhookWindowsHookEx, WindowFromPoint, HHOOK, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL,
    WM_RBUTTONDOWN,
};

use crate::events::DESKTOP_MENU_CLICK;

/// 钩子线程控制消息
enum HookCmd {
    Enable,
    Disable,
    Quit,
}

/// 命中过滤（纯逻辑，可单测）：桌面层窗口类
pub(crate) fn class_is_desktop(cls: &str) -> bool {
    matches!(cls, "Progman" | "WorkerW" | "SHELLDLL_DefView")
}

/// 沿父链向上查找（最多 4 层）判断窗口是否属于桌面层
pub(crate) fn is_desktop_click(hwnd: HWND) -> bool {
    let mut h = hwnd;
    for _ in 0..4 {
        unsafe {
            let mut buf = [0u16; 64];
            let len = GetClassNameW(h, &mut buf);
            let cls = String::from_utf16_lossy(&buf[..len as usize]);
            if class_is_desktop(&cls) {
                return true;
            }
            match GetParent(h) {
                Ok(p) if !p.0.is_null() => h = p,
                _ => break,
            }
        }
    }
    false
}

/// 钩子是否已安装（诊断/测试用）
static HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);
pub fn hook_installed() -> bool {
    HOOK_INSTALLED.load(Ordering::SeqCst)
}

unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && wparam.0 as u32 == WM_RBUTTONDOWN {
        unsafe {
            let data = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            let pt = POINT { x: data.pt.x, y: data.pt.y };
            let target = WindowFromPoint(pt);
            if is_desktop_click(target) {
                // 转成容器窗口局部坐标（事件只发坐标，前端自行画菜单）
                if let Ok(app) = HOOK_APP.try_lock() {
                    if let Some(app) = app.as_ref() {
                        if let Some(hwnd) = super::desktop_layer::container_hwnd(app) {
                            let mut wr = windows::Win32::Foundation::RECT::default();
                            if GetWindowRect(hwnd, &mut wr).is_ok() {
                                use tauri::Emitter;
                                let _ = app.emit(
                                    DESKTOP_MENU_CLICK,
                                    serde_json::json!({
                                        "x": pt.x - wr.left,
                                        "y": pt.y - wr.top,
                                    }),
                                );
                            }
                        }
                    }
                }
                // 吞掉本次右键：系统默认桌面菜单不再弹出
                return LRESULT(1);
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// 事件发送所需 AppHandle（钩子线程通过 try_lock 读取；设置线程持锁时间极短）
static HOOK_APP: std::sync::Mutex<Option<tauri::AppHandle>> = std::sync::Mutex::new(None);

/// 启动钩子线程：安装 LL 钩子 + 消息泵（阻塞；放专用线程）
pub fn spawn(app: tauri::AppHandle, rx: Receiver<HookCmd>) {
    *HOOK_APP.lock().unwrap() = Some(app);
    std::thread::Builder::new()
        .name("lumina-mouse-hook".into())
        .spawn(move || hook_loop(rx))
        .expect("spawn mouse hook thread");
}

fn hook_loop(rx: Receiver<HookCmd>) {
    let mut hook: Option<HHOOK> = None;
    loop {
        // 无钩子时阻塞等命令；有钩子时以消息泵为主、命令为旁路
        let cmd = match hook {
            Some(_) => {
                // 泵一条消息（不阻塞太久），再查命令队列
                unsafe {
                    let mut msg = MSG::default();
                    if GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        windows::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                        windows::Win32::UI::WindowsAndMessaging::DispatchMessageW(&msg);
                    }
                }
                rx.try_recv().ok()
            }
            None => rx.recv().ok(),
        };
        match cmd {
            Some(HookCmd::Enable) => {
                if hook.is_none() {
                    unsafe {
                        use windows::Win32::Foundation::HINSTANCE;
                        let hmod = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
                            .unwrap_or_default();
                        let h = SetWindowsHookExW(
                            WH_MOUSE_LL,
                            Some(mouse_hook),
                            Some(HINSTANCE(hmod.0)),
                            0,
                        );
                        if h.is_err() {
                            log::error!("SetWindowsHookExW failed: {h:?}");
                            continue;
                        }
                        hook = Some(h.unwrap_or_default());
                        HOOK_INSTALLED.store(true, Ordering::SeqCst);
                        log::info!("desktop menu hook installed");
                    }
                }
            }
            Some(HookCmd::Disable) => {
                if let Some(h) = hook.take() {
                    unsafe {
                        let _ = UnhookWindowsHookEx(h);
                    }
                    HOOK_INSTALLED.store(false, Ordering::SeqCst);
                    log::info!("desktop menu hook uninstalled");
                }
            }
            Some(HookCmd::Quit) => {
                if let Some(h) = hook.take() {
                    unsafe {
                        let _ = UnhookWindowsHookEx(h);
                    }
                }
                HOOK_INSTALLED.store(false, Ordering::SeqCst);
                *HOOK_APP.lock().unwrap() = None;
                break;
            }
            None => {}
        }
    }
}

/// 全局命令通道（lib.rs 命令通过 send 控制钩子）
static HOOK_TX: std::sync::OnceLock<std::sync::Mutex<Sender<HookCmd>>> =
    std::sync::OnceLock::new();

/// 启动钩子线程（安装 LL 钩子 + 消息泵）
pub fn init(app: tauri::AppHandle) {
    let (tx, rx) = std::sync::mpsc::channel();
    if HOOK_TX.set(std::sync::Mutex::new(tx)).is_err() {
        log::warn!("hook channel already initialized");
        return;
    }
    spawn(app, rx);
}

fn send(cmd: HookCmd) {
    if let Some(tx) = HOOK_TX.get() {
        let _ = tx.lock().unwrap().send(cmd);
    }
}

pub(crate) fn enable() {
    send(HookCmd::Enable);
}

pub(crate) fn disable() {
    send(HookCmd::Disable);
}

#[cfg(test)]
mod tests {
    use super::class_is_desktop;

    #[test]
    fn desktop_classes_matched() {
        for c in ["Progman", "WorkerW", "SHELLDLL_DefView"] {
            assert!(class_is_desktop(c), "{c} 应命中桌面层");
        }
    }

    #[test]
    fn non_desktop_classes_rejected() {
        for c in [
            "Tauri Window",
            "Chrome_WidgetWin_1",
            "Shell_TrayWnd",
            "SysListView32",
            "",
        ] {
            assert!(!class_is_desktop(c), "{c} 不应命中桌面层");
        }
    }
}
