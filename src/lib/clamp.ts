/**
 * 浮层坐标夹取（纯函数，便于单测）。
 * 小组件/便签位置以逻辑像素持久化，换分辨率或缩放后可能落在视口外；
 * 渲染侧按实际视口把坐标收回，保证元素外框完整可见。
 */

/** 夹取单轴：保持 [0, limit] 内；元素大于视口（limit < 0）时贴齐 0 */
function clampAxis(pos: number, size: number, viewport: number): number {
  return Math.min(Math.max(0, Math.round(pos)), Math.max(0, Math.round(viewport - size)));
}

/** 把 (x, y) 夹取到视口内：元素外框 w×h 完整可见 */
export function clampPosition(
  x: number,
  y: number,
  w: number,
  h: number,
  vw: number,
  vh: number,
): { x: number; y: number } {
  return {
    x: clampAxis(x, w, vw),
    y: clampAxis(y, h, vh),
  };
}
