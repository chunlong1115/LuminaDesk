//! DPI 换算（阶段 2 落地 + 表驱动单测）
//! 物理↔逻辑换算：Per-Monitor V2 下窗口 API 用物理像素，tauri 建窗参数
//! （position/inner_size）与 CSS 用逻辑像素。
//! 产品化沿用 spike 结论：容器/浮层在宿主内的坐标全物理像素。
//! 浮层与容器建窗都要先把 Monitor 的物理几何折算成逻辑像素（建窗即按目标
//! 几何落地，避免 visible(false)+默认尺寸导致 WebView2 dpr 冻结为 1.0）。

/// 物理像素 → 逻辑像素（按显示器 scale_factor 折算；保留小数，供建窗参数使用）
pub fn physical_to_logical(physical: (i32, i32), scale: f64) -> (f64, f64) {
    (physical.0 as f64 / scale, physical.1 as f64 / scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_to_logical_scales() {
        // 125%：1920×1080 物理 → 1536×864 逻辑
        assert_eq!(physical_to_logical((1920, 1080), 1.25), (1536.0, 864.0));
        // 100%：原样
        assert_eq!(physical_to_logical((1920, 1080), 1.0), (1920.0, 1080.0));
        // 负坐标（副屏位置）按比例折算
        assert_eq!(physical_to_logical((-320, 40), 2.0), (-160.0, 20.0));
    }
}
