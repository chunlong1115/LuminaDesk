import { create } from "zustand";

export interface ContainerItem {
  path: string;
  name: string;
  isDir: boolean;
}

interface ContainersState {
  items: ContainerItem[];
  setItems: (items: ContainerItem[]) => void;
}

/** 容器桌面项（阶段 2：只读虚拟列表，来源 notify 去抖刷新） */
export const useContainers = create<ContainersState>((set) => ({
  items: [],
  setItems: (items) => set({ items }),
}));
