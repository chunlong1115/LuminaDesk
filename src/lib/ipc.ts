import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * 全部 invoke/event 的类型化封装（唯一 IPC 入口）。
 * 跨窗口状态一致性：Rust 持久化为唯一数据源，写操作 → invoke → 落盘 → emit 广播。
 */

// ---- 类型 ----

export interface ContainerItem {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  modified: number;
}

export interface LayoutEntry {
  path: string;
  x: number;
  y: number;
}

export interface GroupEntry {
  id: string;
  name: string;
  item_paths: string[];
}

export interface StatsSample {
  cpu_percent: number;
  mem_used_bytes: number;
  mem_total_bytes: number;
}

/** 浮层可点击区（物理像素、客户区坐标；与 Rust utils::hit_test::HitRegion 对齐） */
export interface HitRegion {
  x: number;
  y: number;
  w: number;
  h: number;
}

// ---- 命令 ----

export async function ping(): Promise<string> {
  return invoke("ping");
}

export async function layerStatus(): Promise<string> {
  return invoke("layer_status");
}

export async function layerAttachNow(): Promise<string> {
  return invoke("layer_attach_now");
}

export async function layerDetachNow(): Promise<string> {
  return invoke("layer_detach_now");
}

export async function desktopMenuSet(enabled: boolean): Promise<boolean> {
  return invoke("desktop_menu_set", { enabled });
}

export async function desktopMenuGet(): Promise<boolean> {
  return invoke("desktop_menu_get");
}

export async function statsSubscribe(enabled: boolean): Promise<number> {
  return invoke("stats_subscribe", { enabled });
}

/** 浮层：上报最新可点击区（整体替换，物理像素） */
export async function hitTestSetRegions(regions: HitRegion[]): Promise<void> {
  return invoke("hit_test_set_regions", { regions });
}

/** 浮层：便签/待办输入激活切换（摘/挂 WS_EX_NOACTIVATE + 抢键盘焦点） */
export async function widgetsSetActive(active: boolean): Promise<void> {
  return invoke("widgets_set_active", { active });
}

/** 浮层模式开关：「浮于图标之上」=true 创建 widgets-0 窗口，false 销毁回容器模式 */
export async function systemSetWidgetsAboveIcons(enabled: boolean): Promise<boolean> {
  return invoke("system_set_widgets_above_icons", { enabled });
}

export async function containerListItems(): Promise<ContainerItem[]> {
  return invoke("container_list_items");
}

export async function containerMoveItem(
  path: string,
  x: number,
  y: number,
): Promise<void> {
  return invoke("container_move_item", { path, x, y });
}

export async function containerSaveLayout(entries: LayoutEntry[]): Promise<void> {
  return invoke("container_save_layout", { entries });
}

export async function containerLoadLayout(): Promise<LayoutEntry[]> {
  return invoke("container_load_layout");
}

export async function containerOpenItem(path: string): Promise<void> {
  return invoke("container_open_item", { path });
}

export async function containerRevealItem(path: string): Promise<void> {
  return invoke("container_reveal_item", { path });
}

export async function containerIcon(path: string): Promise<string> {
  return invoke("container_icon", { path });
}

export async function groupList(): Promise<GroupEntry[]> {
  return invoke("group_list");
}

export async function groupCreate(name: string): Promise<GroupEntry[]> {
  return invoke("group_create", { name });
}

export async function groupAddItem(groupId: string, path: string): Promise<void> {
  return invoke("group_add_item", { groupId, path });
}

export async function groupRemoveItem(groupId: string, path: string): Promise<void> {
  return invoke("group_remove_item", { groupId, path });
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
  desktopMenuClick: "lumina://desktop-menu-click",
} as const;

export interface DesktopMenuClickPayload {
  x: number;
  y: number;
}

export function onEvent<T>(name: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(name, (e) => cb(e.payload));
}
