import { create } from "zustand";

interface SettingsState {
  desktopLayerEnabled: boolean;
  widgetsInContainer: boolean;
  containerOpacity: number;
  set: (patch: Partial<SettingsState>) => void;
}

/** 设置（阶段 2 接入 system_set_desktop_layer 持久化；UI 是镜像，Rust 落盘后广播回填） */
export const useSettings = create<SettingsState>((set) => ({
  desktopLayerEnabled: true,
  widgetsInContainer: true,
  containerOpacity: 1,
  set: (patch) => set(patch),
}));
