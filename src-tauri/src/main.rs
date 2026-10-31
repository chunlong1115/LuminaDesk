// 隐藏 Windows 控制台窗口（release 构建无控制台）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    luminadesk_lib::run();
}
