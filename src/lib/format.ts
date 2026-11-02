/** 统计/组件展示格式化（纯函数，前端单测覆盖） */

/** 钳位到 0..=100（CPU 占用等百分比展示统一入口；NaN/Infinity 归一为 0） */
export function clampPct(v: number): number {
  if (!Number.isFinite(v)) return 0;
  return Math.min(100, Math.max(0, v));
}

/** MB → 人类可读：<1GB 显示整数 MB，≥1GB 显示 1 位小数 GB */
export function formatMb(mb: number): string {
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`;
  return `${Math.round(mb)} MB`;
}

/** HH:MM:SS（补零） */
export function formatClock(d: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

/** 距下一秒整点边界的毫秒数（时钟组件对齐秒沿的定时锚点） */
export function msToNextSecond(d: Date): number {
  return 1000 - d.getMilliseconds();
}

/** 日期展示：YYYY年M月D日 周X */
export function formatDate(d: Date): string {
  const days = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];
  return `${d.getFullYear()}年${d.getMonth() + 1}月${d.getDate()}日 ${days[d.getDay()]}`;
}
