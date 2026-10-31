//! 坐标换算纯函数（可单测；spike 实测规则的产品化）。
//!
//! 规则（本机实测）：
//! - 桌面层窗口全程物理像素（Per-Monitor V2 进程 + 未虚拟化坐标）
//! - 子窗口坐标相对宿主客户区：monitor.Bounds − WorkerW 矩形原点 + 期望偏移
//! - 虚拟屏幕可含负坐标（多屏左/上扩展），全部用矩形运算，勿假设原点为 (0,0)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { left, top, right, bottom }
    }

    pub const fn width(&self) -> i32 {
        self.right - self.left
    }

    pub const fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// 逻辑 ↔ 物理（Per-Monitor V2：scale = 显示器 DPI / 96）
pub fn logical_to_physical(logical: i32, scale: f64) -> i32 {
    (logical as f64 * scale).round() as i32
}

pub fn physical_to_logical(physical: i32, scale: f64) -> i32 {
    (physical as f64 / scale).round() as i32
}

/// 子窗口在宿主中的位置：monitor.Bounds − 宿主矩形原点 + 期望偏移（物理像素）
pub fn child_position(monitor: Rect, host: Rect, offset_x: i32, offset_y: i32) -> (i32, i32) {
    (
        monitor.left - host.left + offset_x,
        monitor.top - host.top + offset_y,
    )
}

/// 矩形夹取到监视器工作区内（防分辨率切换后窗口出屏）
pub fn clamp_into(rect: Rect, bounds: Rect) -> Rect {
    let w = rect.width();
    let h = rect.height();
    let bw = bounds.width();
    let bh = bounds.height();
    let left = rect.left.clamp(bounds.left, bounds.right - w.min(bw));
    let top = rect.top.clamp(bounds.top, bounds.bottom - h.min(bh));
    Rect::new(left, top, left + w, top + h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpi_conversion_125() {
        assert_eq!(logical_to_physical(560, 1.25), 700);
        assert_eq!(physical_to_logical(700, 1.25), 560);
        assert_eq!(logical_to_physical(40, 1.25), 50);
    }

    #[test]
    fn child_position_primary_at_origin() {
        let monitor = Rect::new(0, 0, 1536, 864);
        let host = Rect::new(0, 0, 1536, 864);
        assert_eq!(child_position(monitor, host, 40, 40), (40, 40));
    }

    #[test]
    fn child_position_negative_virtual_screen() {
        // 主屏在右，左屏扩展 → 虚拟屏幕原点为负
        let monitor = Rect::new(0, 0, 1536, 864);
        let host = Rect::new(-1920, 0, 1536, 864);
        assert_eq!(child_position(monitor, host, 40, 40), (1960, 40));
    }

    #[test]
    fn clamp_keeps_window_on_screen_after_resolution_drop() {
        let rect = Rect::new(1200, 700, 1200 + 560, 700 + 460);
        let bounds = Rect::new(0, 0, 1280, 720);
        let c = clamp_into(rect, bounds);
        assert_eq!(c.right, 1280);
        assert_eq!(c.bottom, 720);
        assert!(c.width() <= 1280 && c.height() <= 720);
    }

    #[test]
    fn clamp_noop_when_inside() {
        let rect = Rect::new(40, 40, 600, 500);
        let bounds = Rect::new(0, 0, 1536, 864);
        assert_eq!(clamp_into(rect, bounds), rect);
    }
}
