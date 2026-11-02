import { create } from "zustand";

export type WidgetKind = "clock" | "date" | "notes" | "todo" | "cpuMem";

export interface WidgetConfig {
  id: string;
  kind: WidgetKind;
  /** 容器逻辑坐标（125% 缩放下 1536×864）；组件管理页（阶段 3 任务 4）可增删/调位 */
  x: number;
  y: number;
}

/** 默认组件布局：右侧一列 + 底部操作区（持久化 widgets.json 落地前的前端默认值） */
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
}

/** 小组件配置（阶段 3；持久化由 Rust widgets.json 负责——任务 4） */
export const useWidgets = create<WidgetsState>((set) => ({
  widgets: DEFAULT_WIDGETS,
  setWidgets: (widgets) => set({ widgets }),
}));
