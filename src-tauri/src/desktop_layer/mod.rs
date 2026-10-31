//! 桌面层服务入口（阶段 2 从 spike 产品化：<spike-dir>\src-tauri\src\desktop_layer.rs）。
//!
//! spike 已验证的四大关键实现（阶段 2 落地时必须保留）：
//! 1. 启动顺序：先 show → 等 1.5s tao 样式落定 → 再 attach（否则 GWL_STYLE 重应用断父链）
//! 2. 透明：attach 时加 WS_EX_NOREDIRECTIONBITMAP（tao blur-behind 对子窗口失效，黑块根因）
//! 3. find_wallpaper_workerw 枚举优先（父==Progman 且无 SHELLDLL_DefView），找不到才发 0x052C
//! 4. GetParent 判断用 `.is_ok_and(|p| p == host)`（NULL 时 Err(1460) 是陈旧 GetLastError 噪音）

pub mod attach;
pub mod geometry;
pub mod host;
pub mod watchdog;

/// 运行时桌面拓扑
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topology {
    /// Win10 ~ Win11 23H2：壁纸层是顶层 WorkerW，GetParent() == Progman
    Classic,
    /// Win11 24H2+：WorkerW 是 Progman 的直接子窗口（本机不可实测，阶段 2 VM 验证）
    Raised24H2,
    /// 无 Progman（RDP 等会话）
    None,
}
