import type { LayoutEntry } from "../../lib/ipc";

export interface Pos {
  x: number;
  y: number;
}

export interface DragParams {
  cellW: number;
  cellH: number;
  cols: number;
  originX: number;
  originY: number;
  snap: number;
}

/** 默认网格位（未持久化过时按列表序摆放） */
export function gridPosOf(index: number, p: DragParams): Pos {
  return {
    x: p.originX + (index % p.cols) * p.cellW,
    y: p.originY + Math.floor(index / p.cols) * p.cellH,
  };
}

/** 实际渲染位：已存布局优先，否则默认网格位 */
export function resolvePos(
  saved: Pos | undefined,
  index: number,
  p: DragParams,
): Pos {
  return saved ?? gridPosOf(index, p);
}

/** 吸附到网格（8px 步进，防微小抖动导致布局脏数据） */
export function snapPos(pos: Pos, p: DragParams): Pos {
  return {
    x: Math.round(pos.x / p.snap) * p.snap,
    y: Math.round(pos.y / p.snap) * p.snap,
  };
}

/**
 * 拖拽落点 → 全部选中项的批量布局条目：
 * 被拖项落在吸附后的 (nx,ny)，其余选中项保持与它的相对偏移。
 * 未存盘项用其默认网格位作为基准（修复：曾用索引 0 导致起点跳位）。
 */
export function dropEntries(
  selected: { path: string; origin: Pos }[],
  draggedPath: string,
  nx: number,
  ny: number,
): LayoutEntry[] {
  const dragged = selected.find((s) => s.path === draggedPath);
  if (!dragged) return [];
  const dx = nx - dragged.origin.x;
  const dy = ny - dragged.origin.y;
  return selected.map((s) => ({
    path: s.path,
    x: s.origin.x + dx,
    y: s.origin.y + dy,
  }));
}
