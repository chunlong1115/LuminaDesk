import { create } from "zustand";

export interface CpuMemSample {
  cpu: number;
  memUsedMb: number;
  memTotalMb: number;
}

interface StatsState {
  history: CpuMemSample[];
  push: (sample: CpuMemSample) => void;
}

const MAX_POINTS = 60;

/** 统计环形缓冲（60 点，前端零轮询——Rust 侧 1Hz 推送，阶段 3） */
export const useStats = create<StatsState>((set) => ({
  history: [],
  push: (sample) =>
    set((s) => ({ history: [...s.history, sample].slice(-MAX_POINTS) })),
}));
