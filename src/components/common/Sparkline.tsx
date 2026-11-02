/**
 * SVG 迷你图（手绘，不引图表库）。
 * 坐标映射下沉为纯函数 sparkPoints，供单测覆盖。
 */

/** 数据 → polyline 坐标串：x 均布满宽，y 以 0..=max 归一（max 值贴顶） */
export function sparkPoints(data: number[], w: number, h: number): string {
  if (data.length < 2) return "";
  const max = Math.max(...data, 1);
  return data
    .map(
      (v, i) =>
        `${(i / (data.length - 1)) * w},${h - (v / max) * h}`,
    )
    .join(" ");
}

export function Sparkline({ data }: { data: number[] }) {
  const w = 100;
  const h = 30;
  const pts = sparkPoints(data, w, h);
  if (!pts) return null;
  return (
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} style={{ display: "block" }}>
      <polyline points={pts} fill="none" stroke="#7db4ff" strokeWidth="1.5" />
    </svg>
  );
}
