import { useEffect } from "react";
import { WidgetsLayer } from "../../components/widgets/WidgetsLayer";
import { hitTestSetRegions, type HitRegion } from "../../lib/ipc";
import { regionsFromRects, sameRegions } from "../../lib/hitRegions";

/** 上报去抖（拖拽/缩放连续 mutation 合并） */
const REPORT_DEBOUNCE_MS = 50;

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

  return (
    <div style={{ background: "transparent" }}>
      <WidgetsLayer />
    </div>
  );
}
