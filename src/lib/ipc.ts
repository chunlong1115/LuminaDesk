import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * 全部 invoke/event 的类型化封装（唯一 IPC 入口）。
 * 跨窗口状态一致性：Rust 持久化为唯一数据源，写操作 → invoke → 落盘 → emit 广播。
 */

// ---- 命令（阶段 2-4 逐步补充实现） ----

export async function ping(): Promise<string> {
  return invoke("ping");
}

// ---- 事件 ----

export const EVENTS = {
  layerChanged: "lumina://layer-changed",
  statsSample: "lumina://stats-sample",
  displayChanged: "lumina://display-changed",
  fsChanged: "lumina://fs-changed",
  containersChanged: "lumina://containers-changed",
  widgetsChanged: "lumina://widgets-changed",
  reattachLimit: "lumina://reattach-limit",
} as const;

export function onEvent<T>(name: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(name, (e) => cb(e.payload));
}
