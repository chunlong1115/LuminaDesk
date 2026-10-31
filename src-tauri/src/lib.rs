//! LuminaDesk 主库：插件注册、托盘、窗口创建、命令分发。
//! 阶段 1 骨架：托盘 + 控制窗懒创建 + 持久化/shell 监听接线；
//! 容器挂载链路（desktop_layer）阶段 2 落地。

mod commands;
mod desktop_layer;
mod events;
mod persistence;
mod shell_window;
mod stats;
mod utils;

use tauri::{AppHandle, Manager};

/// 控制窗口：关闭即销毁（destroy，非 hide）——省内存（WebView2 视图不驻留）
fn create_control_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("control") {
        w.show()?;
        w.set_focus()?;
        return Ok(());
    }
    tauri::WebviewWindowBuilder::new(
        app,
        "control",
        tauri::WebviewUrl::App("index.html?role=control".into()),
    )
    .title("LuminaDesk")
    .inner_size(860.0, 640.0)
    .additional_browser_args(
        // 防节流三件套 + 关闭遮挡追踪（桌面层窗口必需；spike S4 实测有效）
        // 注意：同一应用内所有窗口的 additionalBrowserArgs 必须完全一致
        //（Tauri 2 单一共享 WebView2 环境按首个窗口参数初始化——spike 0x8007139F 根因）
        "--disable-background-timer-throttling --disable-backgrounding-occluded-windows --disable-renderer-backgrounding --disable-features=CalculateNativeWinOcclusion",
    )
    .build()?;
    Ok(())
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::tray::TrayIconBuilder;

    let open = MenuItemBuilder::with_id("open-control", "打开控制台").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
    let menu = MenuBuilder::new(app).item(&open).item(&quit).build()?;

    let mut tray = TrayIconBuilder::with_id("lumina-main")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open-control" => {
                if let Err(e) = create_control_window(app) {
                    log::error!("open control failed: {e}");
                }
            }
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[tauri::command]
fn ping() -> String {
    "pong".into()
}

pub fn run() {
    let _app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_single_instance::init(|app, _args, _cwd| {
                // 第二实例：唤起已有控制窗口
                if let Err(e) = create_control_window(app) {
                    log::error!("single-instance focus failed: {e}");
                }
            }),
        )
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("luminadesk".into()),
                    }),
                ])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![ping])
        .setup(|app| {
            // 1. 隐藏 shell 监听窗口（TaskbarCreated/DisplayChange/SettingChange/PowerBroadcast）
            //    阶段 2 接入看门狗；当前仅启动线程防后续行为偏差
            let _rx = shell_window::spawn_shell_listener();

            // 2. 托盘
            let handle = app.handle().clone();
            build_tray(&handle)?;

            // 3. 阶段 1：自动打开控制窗口便于验收；产品化改为纯托盘懒加载
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = create_control_window(&handle) {
                    log::error!("startup control window failed: {e}");
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
