/** SVG 迷你图（阶段 3 完善样式；不引入图表库） */
export function Sparkline({ data }: { data: number[] }) {
  if (data.length < 2) return null;
  const w = 100;
  const h = 30;
  const max = Math.max(...data, 1);
  const pts = data
    .map((v, i) => `${(i / (data.length - 1)) * w},${h - (v / max) * h}`)
    .join(" ");
  return (
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`}>
      <polyline points={pts} fill="none" stroke="#7db4ff" strokeWidth="1.5" />
    </svg>
  );
}
