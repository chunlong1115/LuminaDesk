//! 小组件层点击穿透（WM_NCHITTEST）纯逻辑与共享数据。
//! 前端上报可点击矩形区（物理像素、客户区坐标），Rust 侧共享列表供窗口过程命中判定：
//! 非组件区返回 HTTRANSPARENT 透传给下层窗口（桌面图标），组件区正常接收。
//! 窗口子类化在 widgets_layer（Win32 密集，与 host/attach 同约定不做单测）。

use std::sync::RwLock;

/// 可点击矩形区（物理像素，客户区坐标）
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HitRegion {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl HitRegion {
    /// 点命中判定：左/上边界含，右/下边界不含
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

/// 共享可点击区（WM_NCHITTEST 高频读、前端布局变更低频写 → RwLock）
static REGIONS: RwLock<Vec<HitRegion>> = RwLock::new(Vec::new());

/// 前端上报最新可点击区（整体替换）
pub fn set_regions(regions: Vec<HitRegion>) {
    *REGIONS.write().unwrap() = regions;
}

/// 当前可点击区快照（窗口过程命中判定用）
pub fn regions_snapshot() -> Vec<HitRegion> {
    REGIONS.read().unwrap().clone()
}

/// 命中判定纯函数：点（客户区坐标）是否落在任一可点击区
pub fn hit_at(regions: &[HitRegion], x: i32, y: i32) -> bool {
    regions.iter().any(|r| r.contains(x, y))
}

/// WM_NCHITTEST lParam 拆包：屏幕坐标按 16 位有符号分量解析。
/// 多显示器负坐标必须符号扩展（否则左侧/上方副屏的判定全错）。
pub fn unpack_lparam(lparam: isize) -> (i32, i32) {
    let x = (lparam as u16) as i16 as i32;
    let y = ((lparam >> 16) as u16) as i16 as i32;
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: i32, y: i32, w: i32, h: i32) -> HitRegion {
        HitRegion { x, y, w, h }
    }

    #[test]
    fn contains_boundaries() {
        let region = r(10, 20, 30, 40);
        assert!(region.contains(10, 20)); // 左/上边界含
        assert!(region.contains(39, 59)); // 右/下边界前一点
        assert!(!region.contains(40, 60)); // 右/下边界不含
        assert!(!region.contains(9, 20));
        assert!(!region.contains(10, 19));
    }

    #[test]
    fn hit_at_multiple_regions() {
        let regions = vec![r(0, 0, 10, 10), r(100, 100, 50, 50)];
        assert!(hit_at(&regions, 5, 5));
        assert!(hit_at(&regions, 149, 149));
        assert!(!hit_at(&regions, 50, 50));
        assert!(!hit_at(&[], 0, 0)); // 空列表全穿透
    }

    #[test]
    fn unpack_lparam_positive_and_negative() {
        assert_eq!(unpack_lparam(0x000A_0014), (20, 10));
        // y = -10（左侧/上方副屏负坐标），符号扩展还原
        assert_eq!(unpack_lparam(((-10i32) << 16) as isize + 5), (5, -10));
    }

    #[test]
    fn regions_set_and_snapshot() {
        set_regions(vec![r(1, 2, 3, 4)]);
        assert_eq!(regions_snapshot(), vec![r(1, 2, 3, 4)]);
        set_regions(vec![]);
        assert!(regions_snapshot().is_empty());
    }
}
