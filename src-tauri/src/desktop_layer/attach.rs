//! SetParent 执行（阶段 2 从 spike attach/detach 移植）
//! TODO(阶段2): WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP → POPUP 换 CHILD →
//!               SetParent + 双重校验 → 物理坐标换算 SetWindowPos
