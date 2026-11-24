import { useEffect, useRef } from "react";
import { WidgetsLayer } from "../../components/widgets/WidgetsLayer";
import { clampPosition } from "../../lib/clamp";
import { hitTestSetRegions, widgetsUpsert, type HitRegion } from "../../lib/ipc";
import { regionsFromRects, sameRegions } from "../../lib/hitRegions";
import { useNotes } from "../../stores/notes";
import { useWidgets } from "../../stores/widgets";

/** 上报去抖（拖拽/缩放连续 mutation 合并） */
const REPORT_DEBOUNCE_MS = 50;

/** 夹取留痕（排查换分辨率/缩放后的布局问题） */
function logClamp(id: string, from: { x: number; y: number }, to: { x: number; y: number }) {
  console.info(`[lumina] clamp ${id}: (${from.x},${from.y}) -> (${to.x},${to.y})`);
}

/**
 * 小组件层窗口（「浮于图标之上」模式启用时使用）：
 * 全窗透明，渲染 WidgetsLayer；收集 [data-hit-region] 元素矩形
 * （CSS px × devicePixelRatio → 物理 px）上报 Rust 命中区，
 * WM_NCHITTEST 据此透传非组件区点击（桌面图标）、组件区正常接收。
 */
export function WidgetsPage() {
  useEffect(() => {
    let lastRegions: HitRegion[] | null = null;
    let timer: number | undefined;

    const report = () => {
      const nodes = document.querySelectorAll("[data-hit-region]");
      const rects = Array.from(nodes).map((n) => n.getBoundingClientRect());
      const regions = regionsFromRects(rects, window.devicePixelRatio);
      if (lastRegions !== null && sameRegions(lastRegions, regions)) return;
      lastRegions = regions;
      void hitTestSetRegions(regions);
    };

    const schedule = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(report, REPORT_DEBOUNCE_MS);
    };

    // 拖拽/缩放改 style、增删改 class → mutation；窗口尺寸变化 → resize
    const observer = new MutationObserver(schedule);
    observer.observe(document.body, {
      subtree: true,
      childList: true,
      attributes: true,
      attributeFilter: ["style", "class"],
    });
    window.addEventListener("resize", schedule);
    report(); // 首帧立即上报

    return () => {
      window.clearTimeout(timer);
      observer.disconnect();
      window.removeEventListener("resize", schedule);
    };
  }, []);

  // 坐标夹取：位置按逻辑像素持久化，换分辨率/缩放后可能落在视口外。卡片尺寸
  // 由 CSS 决定（如待办 264×320），故按渲染后的实际外框测量；便签自带 w/h。
  // 每次窗口会话每个 id 只校正一次：widgetsUpsert 会广播回本窗口，无此闸门
  // 会形成「校正 → 广播 → 再校正」的循环
  const widgets = useWidgets((s) => s.widgets);
  const notes = useNotes((s) => s.notes);
  const clamped = useRef(new Set<string>());

  useEffect(() => {
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    if (vw <= 0 || vh <= 0) return;

    for (const w of widgets) {
      if (w.kind === "notes" || clamped.current.has(w.id)) continue;
      const el = document.querySelector<HTMLElement>(`[data-widget-id="${w.id}"]`);
      if (!el) continue; // 尚未挂载（水合中），等下次列表变更
      const rect = el.getBoundingClientRect();
      const p = clampPosition(w.x, w.y, rect.width, rect.height, vw, vh);
      if (p.x === w.x && p.y === w.y) continue;
      clamped.current.add(w.id);
      logClamp(w.id, w, p);
      void widgetsUpsert({ ...w, ...p }).catch((e) => {
        console.error("widget clamp persist failed", e);
      });
    }

    for (const n of notes) {
      if (clamped.current.has(n.id)) continue;
      const p = clampPosition(n.x, n.y, n.w, n.h, vw, vh);
      if (p.x === n.x && p.y === n.y) continue;
      clamped.current.add(n.id);
      logClamp(n.id, n, p);
      useNotes.getState().update(n.id, p);
    }
  }, [widgets, notes]);

  return (
    <div style={{ background: "transparent" }}>
      <WidgetsLayer />
    </div>
  );
}
