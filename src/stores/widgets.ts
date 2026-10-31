import { create } from "zustand";

export interface WidgetConfig {
  id: string;
  kind: "clock" | "date" | "notes" | "todo" | "cpuMem";
  x: number;
  y: number;
}

interface WidgetsState {
  widgets: WidgetConfig[];
  setWidgets: (widgets: WidgetConfig[]) => void;
}

/** 小组件配置（阶段 3） */
export const useWidgets = create<WidgetsState>((set) => ({
  widgets: [],
  setWidgets: (widgets) => set({ widgets }),
}));
