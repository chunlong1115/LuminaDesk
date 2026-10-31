//! 事件常量（Rust 侧 emit，前端 listen；跨窗口状态一致性以 Rust 持久化为唯一数据源）
pub const LAYER_CHANGED: &str = "lumina://layer-changed";
pub const STATS_SAMPLE: &str = "lumina://stats-sample";
pub const DISPLAY_CHANGED: &str = "lumina://display-changed";
pub const FS_CHANGED: &str = "lumina://fs-changed";
pub const CONTAINERS_CHANGED: &str = "lumina://containers-changed";
pub const WIDGETS_CHANGED: &str = "lumina://widgets-changed";
pub const REATTACH_LIMIT: &str = "lumina://reattach-limit";
pub const DESKTOP_MENU_CLICK: &str = "lumina://desktop-menu-click";
