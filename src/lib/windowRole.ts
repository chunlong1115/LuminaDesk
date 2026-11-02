import { widgetsSetActive } from "./ipc";

/**
 * 窗口角色：URL ?role= 区分——widgets=浮层小组件窗口、control=控制窗口、容器窗口无 role。
 * 浮层窗口 NOACTIVATE 拿不到键盘输入：输入框聚焦时摘免激活抢焦点（Rust set_active），
 * 失焦恢复（WM_KILLFOCUS 双保险）。其他窗口（容器/控制）无需处理。
 */
export function isWidgetsRole(search: string): boolean {
  return new URLSearchParams(search).get("role") === "widgets";
}

/** 仅浮层窗口转发激活切换（便签/待办输入聚焦/失焦） */
export function setActiveIfWidgets(search: string, active: boolean): void {
  if (isWidgetsRole(search)) void widgetsSetActive(active);
}
