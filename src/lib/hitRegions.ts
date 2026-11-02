import type { HitRegion } from "./ipc";

/**
 * 浮层命中区上报纯函数：DOM 矩形（CSS px）× dpr → 物理像素整数区。
 * Rust 侧零 DPI 换算；左/上向下取整、右/下向上取整——宁可外扩 1px 不漏点击。
 * WM_NCHITTEST 判定含左/上、不含右/下（utils/hit_test::contains）。
 */
export function regionsFromRects(
  rects: { x: number; y: number; width: number; height: number }[],
  dpr: number,
): HitRegion[] {
  return rects.map((r) => {
    const x = Math.floor(r.x * dpr);
    const y = Math.floor(r.y * dpr);
    const right = Math.ceil((r.x + r.width) * dpr);
    const bottom = Math.ceil((r.y + r.height) * dpr);
    return { x, y, w: right - x, h: bottom - y };
  });
}

/** 上报去重：区域列表逐项相等（顺序敏感；DOM 顺序稳定） */
export function sameRegions(a: HitRegion[], b: HitRegion[]): boolean {
  if (a.length !== b.length) return false;
  return a.every((r, i) => {
    const o = b[i]!;
    return r.x === o.x && r.y === o.y && r.w === o.w && r.h === o.h;
  });
}
