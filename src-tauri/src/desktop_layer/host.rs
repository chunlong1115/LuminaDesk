//! 宿主定位与拓扑探测（阶段 2 从 spike find_wallpaper_workerw / detect_topology 移植）
//! TODO(阶段2): 枚举优先 + 0x052C 按需发送；RDP 检测（SM_REMOTESESSION）挂起桌面层

use super::Topology;

/// 运行时拓扑探测（阶段 2 落地：Progman GWL_EXSTYLE 含 WS_EX_NOREDIRECTIONBITMAP 即 24H2+）
pub fn detect_topology() -> Topology {
    Topology::None
}
