import { create } from "zustand";

export interface PlanPreviewItem {
  source: string;
  target: string;
  conflict: boolean;
}

interface OrganizeState {
  preview: PlanPreviewItem[];
  setPreview: (items: PlanPreviewItem[]) => void;
}

/** 文件整理预览（阶段 4：v1 仅虚拟分组/文件夹映射，不移动用户原始文件） */
export const useOrganize = create<OrganizeState>((set) => ({
  preview: [],
  setPreview: (preview) => set({ preview }),
}));
