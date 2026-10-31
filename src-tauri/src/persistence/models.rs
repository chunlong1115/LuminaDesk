//! 持久化模型（阶段 2-4 逐步填充；字段变更注意 serde 默认值兼容）
use serde::{Deserialize, Serialize};

/// 应用设置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    /// 桌面层开关（false = 贴底普通窗口降级）
    pub desktop_layer_enabled: bool,
    /// 小组件默认合并进容器窗口（省内存）；false = 浮于图标之上独立层
    pub widgets_in_container: bool,
    /// 容器不透明度 0.0-1.0
    pub container_opacity: f64,
    /// 主题（v1 仅 dark）
    pub theme: String,
}

/// 容器布局（物理像素存盘，加载时按当前 DPI 换算）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContainerLayout {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 虚拟分组
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VirtualGroup {
    pub id: String,
    pub name: String,
    pub item_paths: Vec<String>,
}
