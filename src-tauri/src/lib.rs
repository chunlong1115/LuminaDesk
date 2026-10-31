//! LuminaDesk 主库：插件注册、托盘、窗口创建、桌面层接线、命令分发。

mod commands;
mod desktop_layer;
mod events;
mod fs_watch;
mod persistence;
mod shell_window;
mod stats;
mod utils;

use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

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
    // 与容器窗口参数完全一致（共享 WebView2 环境约束——spike 0x8007139F 根因）
    .additional_browser_args(desktop_layer::DESKTOP_LAYER_ARGS)
    .build()?;
    Ok(())
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::tray::TrayIconBuilder;

    let open = MenuItemBuilder::with_id("open-control", "打开控制台").build(app)?;
    let attach = MenuItemBuilder::with_id("layer-attach", "重新挂载桌面层").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&open)
        .item(&attach)
        .item(&quit)
        .build()?;

    let mut tray = TrayIconBuilder::with_id("lumina-main")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open-control" => {
                if let Err(e) = create_control_window(app) {
                    log::error!("open control failed: {e}");
                }
            }
            "layer-attach" => {
                if let Err(e) = desktop_layer::attach_container(app, "manual") {
                    log::warn!("manual attach failed: {e}");
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

/// 诊断滴答计数（S4 回归基础设施）：容器页在 ?diag=1 时每秒上报一次，
/// 验证遮挡下 JS 定时器不节流。仅诊断路径，产品零开销。
static DIAG_TICKS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

#[tauri::command]
fn diag_tick() -> u32 {
    let n = DIAG_TICKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    println!("[lumina-diag] tick={n}");
    n
}

#[tauri::command]
fn layer_status(app: AppHandle) -> String {
    desktop_layer::layer_status(&app)
}

#[tauri::command]
fn layer_attach_now(app: AppHandle) -> Result<String, String> {
    desktop_layer::attach_container(&app, "manual")
}

#[tauri::command]
fn layer_detach_now(app: AppHandle) -> Result<String, String> {
    desktop_layer::detach_container(&app)
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
        .invoke_handler(tauri::generate_handler![
            ping,
            diag_tick,
            layer_status,
            layer_attach_now,
            layer_detach_now,
            commands::container_list_items,
            commands::container_move_item,
            commands::container_save_layout,
            commands::container_load_layout,
            commands::container_open_item,
            commands::container_reveal_item,
            commands::container_icon,
            commands::group_list,
            commands::group_create,
            commands::group_add_item,
            commands::group_remove_item
        ])
        .setup(|app| {
            // 1. 隐藏 shell 监听窗口（TaskbarCreated/DisplayChange/SettingChange/PowerBroadcast）
            let rx = shell_window::spawn_shell_listener();

            // 2. 看门狗线程（5s 校验 + 事件驱动重挂）
            let handle = app.handle().clone();
            std::thread::Builder::new()
                .name("layer-watchdog".into())
                .spawn(move || desktop_layer::watchdog::run(handle, rx))?;

            // 3. 创建容器窗口（主显示器）+ 初始挂载
            //    spike 修复 1：先 show 等 tao 一次性样式应用落定（1.5s），再 attach——实测零断裂
            if desktop_layer::create_container_window(app.handle(), "container-0").is_none() {
                log::error!("initial container window creation failed");
                return Ok(());
            }
            let handle = app.handle().clone();
            std::thread::Builder::new()
                .name("layer-attach".into())
                .spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(600));
                    if let Some(w) = handle.get_webview_window(&desktop_layer::container_label()) {
                        let _ = w.show();
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                    match desktop_layer::attach_container(&handle, "startup") {
                        Ok(_) => {
                            let status = desktop_layer::layer_status(&handle);
                            log::info!("startup status:\n{status}");
                        }
                        Err(e) => log::warn!("startup attach failed: {e}"),
                    }
                })?;

            // 4. 桌面目录文件监听（300ms 去抖 → fs-changed 事件）
            match commands::desktop_dir_path() {
                Ok(dir) => {
                    if let Err(e) = fs_watch::spawn(app.handle().clone(), dir) {
                        log::warn!("fs watcher start failed: {e}");
                    }
                }
                Err(e) => log::warn!("desktop dir resolve failed: {e}"),
            }

            // 5. 托盘
            let handle = app.handle().clone();
            build_tray(&handle)?;

            // 6. 打开控制窗口（产品化改为纯托盘懒加载；开发期默认打开便于验收）
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
