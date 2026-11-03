import type { WidgetConfig, WidgetKind } from "../../lib/ipc";

/** 组件类型中文名（管理面板展示） */
export const WIDGET_KIND_LABELS: Record<WidgetKind, string> = {
  clock: "时钟",
  date: "日期",
  cpuMem: "CPU/内存",
  notes: "便签",
  todo: "待办",
};

/** 新增组件默认位置：按现有数量阶梯偏移，避免与现有组件完全重叠 */
export function nextWidgetPos(existing: WidgetConfig[]): { x: number; y: number } {
  const n = existing.length;
  return { x: 100 + (n % 6) * 60, y: 100 + Math.floor(n / 6) * 60 };
}

let nextId = 0;

/** 新增组件 id：kind + 时间戳 + 计数（同 kind 多次添加不冲突） */
export function newWidgetId(kind: WidgetKind): string {
  nextId += 1;
  return `${kind}-${Date.now()}-${nextId}`;
}

/** 位置输入解析：整数转数字；空白/非法 → 回退原值（不打断用户输入） */
export function parsePos(value: string, fallback: number): number {
  const n = Number(value);
  if (value.trim() === "" || !Number.isFinite(n) || !Number.isInteger(n)) return fallback;
  return n;
}
