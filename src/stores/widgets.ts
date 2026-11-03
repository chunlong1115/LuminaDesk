import { create } from "zustand";
import { EVENTS, onEvent, widgetsList, type WidgetConfig, type WidgetKind } from "../lib/ipc";

export type { WidgetConfig, WidgetKind };

/** 默认组件布局：Rust widgets.json 落地前的水合前兜底（与 commands::widgets::default_widgets 对齐） */
export const DEFAULT_WIDGETS: WidgetConfig[] = [
  { id: "clock", kind: "clock", x: 1200, y: 28 },
  { id: "date", kind: "date", x: 1200, y: 106 },
  { id: "cpuMem", kind: "cpuMem", x: 1200, y: 170 },
  { id: "notes", kind: "notes", x: 1020, y: 340 },
  { id: "todo", kind: "todo", x: 1256, y: 340 },
];

interface WidgetsState {
  widgets: WidgetConfig[];
  setWidgets: (widgets: WidgetConfig[]) => void;
  /** 从 Rust 加载配置并订阅 widgets-changed 广播（幂等；跨窗口配置同步） */
  hydrate: () => void;
}

let hydrated = false;

/** 仅测试用：重置水合状态 */
export function _resetForTest(): void {
  hydrated = false;
}

/** 组件配置状态（Rust widgets.json 为唯一数据源，本 store 是各窗口的镜像缓存：
 *  管理页写 → Rust 落盘 → emit 广播 → 容器/浮层窗口经监听更新） */
export const useWidgets = create<WidgetsState>((set) => ({
  widgets: DEFAULT_WIDGETS,
  setWidgets: (widgets) => set({ widgets }),
  hydrate: () => {
    if (hydrated) return;
    hydrated = true;
    void (async () => {
      try {
        const list = await widgetsList();
        set({ widgets: list });
        await onEvent<WidgetConfig[]>(EVENTS.widgetsChanged, (payload) => {
          set({ widgets: payload });
        });
      } catch (e) {
        console.error("widgets hydrate failed", e);
        hydrated = false; // 失败允许重试
      }
    })();
  },
}));
