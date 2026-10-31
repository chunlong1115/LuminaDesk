/**
 * DPI 换算（前端侧；Rust 侧对应 utils/dpi.rs，阶段 2 落地 + 测试）
 * 规则（spike 实测）：窗口 API 用物理像素，创建参数/CSS 用逻辑像素，
 * 布局持久化存物理像素，加载时按当前 DPI 换算。
 */
export function logicalToPhysical(logical: number, scaleFactor: number): number {
  return Math.round(logical * scaleFactor);
}

export function physicalToLogical(physical: number, scaleFactor: number): number {
  return Math.round(physical / scaleFactor);
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
