import { create } from "zustand";
import { clampPct } from "../lib/format";
import { EVENTS, onEvent, statsSubscribe, type StatsSample } from "../lib/ipc";

export interface CpuMemSample {
  cpu: number;
  memUsedMb: number;
  memTotalMb: number;
}

interface StatsState {
  history: CpuMemSample[];
  push: (sample: CpuMemSample) => void;
  /** 订阅：计数 0→1 时启动 Rust 采样线程并监听事件 */
  subscribe: () => void;
  /** 退订：计数归零时停摆采样线程并摘除监听（幂等） */
  unsubscribe: () => void;
}

const MAX_POINTS = 60;

/** Rust 载荷 → 前端样本：CPU 钳位 0..=100，字节 → MB */
export function toSample(s: StatsSample): CpuMemSample {
  return {
    cpu: clampPct(s.cpu_percent),
    memUsedMb: Math.round(s.mem_used_bytes / 1024 / 1024),
    memTotalMb: Math.round(s.mem_total_bytes / 1024 / 1024),
  };
}

/** 订阅引用计数与事件监听句柄（模块级；跨组件共享一份 Rust 采样线程） */
let refCount = 0;
let unlisten: (() => void) | null = null;

/** 仅测试用：重置订阅状态 */
export function _resetForTest(): void {
  refCount = 0;
  unlisten = null;
}

/** 统计环形缓冲（60 点，前端零轮询——Rust 侧 1Hz 推送） */
export const useStats = create<StatsState>((set) => ({
  history: [],
  push: (sample) =>
    set((s) => ({ history: [...s.history, sample].slice(-MAX_POINTS) })),
  subscribe: () => {
    refCount += 1;
    if (refCount !== 1) return;
    void (async () => {
      try {
        await statsSubscribe(true);
        if (refCount === 0) {
          // 订阅在途时已全部退订（StrictMode 双挂载等）→ 立即补退订
          void statsSubscribe(false).catch(() => {});
          return;
        }
        const off = await onEvent<StatsSample>(EVENTS.statsSample, (p) => {
          useStats.getState().push(toSample(p));
        });
        if (refCount === 0) {
          // 监听建立完成前已全部退订 → 摘除并停摆
          off();
          void statsSubscribe(false).catch(() => {});
          return;
        }
        unlisten = off;
      } catch (e) {
        console.error("stats subscribe failed", e);
        if (refCount > 0) refCount -= 1;
      }
    })();
  },
  unsubscribe: () => {
    if (refCount === 0) return;
    refCount -= 1;
    if (refCount !== 0) return;
    unlisten?.();
    unlisten = null;
    void statsSubscribe(false).catch(() => {});
  },
}));
