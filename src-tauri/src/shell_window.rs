//! 隐藏真实窗口的消息泵线程（从 spike 移植，已验证）。
//!
//! TaskbarCreated / WM_DISPLAYCHANGE / WM_SETTINGCHANGE / WM_POWERBROADCAST
//! 等广播消息必须由真实 HWND 接收（HWND_MESSAGE 收不到广播），
//! 专用线程跑 GetMessageW 泵，事件经 channel 转发给看门狗。

use std::sync::mpsc::Sender;
use std::thread;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, MSG, WNDCLASSW,
    WM_DESTROY, WM_DISPLAYCHANGE, WM_POWERBROADCAST, WM_SETTINGCHANGE,
};

const CLASS_NAME: PCWSTR = w!("LuminaDeskShellListener");

/// 广播事件（看门狗据此触发重挂）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellEvent {
    TaskbarCreated,
    DisplayChange,
    SettingChange,
    PowerBroadcast,
}

/// 注册类 + 创建隐藏窗口 + 消息泵（阻塞调用，须放专用线程）
fn listener_main(tx: Sender<ShellEvent>) -> Result<(), String> {
    unsafe {
        let hinst = GetModuleHandleW(None).map_err(|e| format!("GetModuleHandleW: {e}"))?;
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(shell_wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst.into(),
            hIcon: Default::default(),
            hCursor: Default::default(),
            hbrBackground: Default::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: CLASS_NAME,
        };
        if RegisterClassW(&wc) == 0 {
            // 类已注册（重启线程等）时继续
        }
        let hwnd = CreateWindowExW(
            Default::default(),
            CLASS_NAME,
            w!(""),
            Default::default(),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            Some(hinst.into()),
            None,
        )
        .map_err(|e| format!("CreateWindowExW: {e}"))?;

        // 窗口过程与线程绑定：把 tx 存到 GWLP_USERDATA
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowLongPtrW, GWLP_USERDATA};
        let boxed = Box::new(tx);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(boxed) as isize);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        Ok(())
    }
}

unsafe extern "system" fn shell_wndproc(
    hwnd: HWND,
    msg: u32,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWLP_USERDATA};
    const REGISTERED_TASKBAR: &str = "TaskbarCreated";

    if msg == WM_DESTROY {
        let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Sender<ShellEvent>;
        if !p.is_null() {
            drop(Box::from_raw(p));
        }
        return DefWindowProcW(hwnd, msg, _wparam, _lparam);
    }

    // TaskbarCreated 是注册消息，无法用常量匹配——注册后比较
    static REG: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    let reg = *REG.get_or_init(|| {
        use windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW;
        let mut buf = [0u16; 32];
        for (i, c) in REGISTERED_TASKBAR.encode_utf16().enumerate() {
            buf[i] = c;
        }
        unsafe { RegisterWindowMessageW(PCWSTR(buf.as_ptr())) }
    });

    let tx_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Sender<ShellEvent> };
    let event = if msg == reg {
        Some(ShellEvent::TaskbarCreated)
    } else if msg == WM_SETTINGCHANGE {
        Some(ShellEvent::SettingChange)
    } else if msg == WM_DISPLAYCHANGE {
        Some(ShellEvent::DisplayChange)
    } else if msg == WM_POWERBROADCAST {
        Some(ShellEvent::PowerBroadcast)
    } else {
        None
    };
    if let (Some(ev), true) = (event, !tx_ptr.is_null()) {
        unsafe {
            let _ = (*tx_ptr).send(ev);
        }
    }

    unsafe { DefWindowProcW(hwnd, msg, _wparam, _lparam) }
}

/// 启动监听线程，返回事件接收端
pub fn spawn_shell_listener() -> std::sync::mpsc::Receiver<ShellEvent> {
    let (tx, rx) = std::sync::mpsc::channel();
    thread::Builder::new()
        .name("lumina-shell-listener".into())
        .spawn(move || {
            if let Err(e) = listener_main(tx) {
                eprintln!("shell listener exited: {e}");
            }
        })
        .expect("spawn shell listener");
    rx
}
