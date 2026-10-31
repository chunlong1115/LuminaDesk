import { invoke } from "@tauri-apps/api/core";

/**
 * 诊断模式（S4 回归基础设施）：?diag=1 时容器页每秒上报滴答，
 * 验证遮挡下 JS 定时器不被 Chromium 节流（防节流 flags 有效性）。
 * 仅 debug 构建的容器 URL 携带该参数（Rust 侧生成），release 零开销。
 */
export function shouldEnableDiag(search: string): boolean {
  try {
    return new URLSearchParams(search).get("diag") === "1";
  } catch {
    return false;
  }
}

export async function diagTick(): Promise<number> {
  return invoke("diag_tick");
}
