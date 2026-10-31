import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  containerIcon,
  containerListItems,
  containerLoadLayout,
  containerOpenItem,
  containerRevealItem,
  containerSaveLayout,
  EVENTS,
  groupAddItem,
  groupCreate,
  groupList,
  onEvent,
  type ContainerItem,
  type GroupEntry,
} from "../../lib/ipc";
import { diagTick, shouldEnableDiag } from "../../lib/diag";
import { dropEntries, resolvePos, snapPos, type Pos } from "./dragMath";
import styles from "./ContainerPage.module.css";

const DRAG: import("./dragMath").DragParams = {
  cellW: 104,
  cellH: 96,
  cols: 5,
  originX: 16,
  originY: 16,
  snap: 8,
};

interface MenuState {
  x: number;
  y: number;
  path: string | null; // null = 空白区菜单（新建分组）
}

/** 桌面容器（壁纸层、图标之下）：只读虚拟列表 + 拖拽摆放 + 虚拟分组 */
export function ContainerPage() {
  const [items, setItems] = useState<ContainerItem[]>([]);
  const [positions, setPositions] = useState<Map<string, Pos>>(new Map());
  const [icons, setIcons] = useState<Map<string, string>>(new Map());
  const [iconErrors, setIconErrors] = useState<Map<string, string>>(new Map());
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [groups, setGroups] = useState<GroupEntry[]>([]);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [groupSubmenu, setGroupSubmenu] = useState(false);

  const dragRef = useRef<{
    startX: number;
    startY: number;
    path: string;
    origin: Pos;
    cur: Pos;
    moved: boolean;
  } | null>(null);
  const bandRef = useRef<{ x0: number; y0: number; x1: number; y1: number } | null>(
    null,
  );
  const pageRef = useRef<HTMLDivElement>(null);
  const [dragPos, setDragPos] = useState<Pos | null>(null);
  const [band, setBand] = useState<{ x0: number; y0: number; x1: number; y1: number } | null>(null);

  // 初始加载 + fs-changed 刷新（300ms 去抖由 Rust 侧完成）
  const refresh = useCallback(async () => {
    try {
      const [list, layout, gs] = await Promise.all([
        containerListItems(),
        containerLoadLayout(),
        groupList(),
      ]);
      setItems(list);
      setGroups(gs);
      setPositions(new Map(layout.map((e) => [e.path, { x: e.x, y: e.y }])));
    } catch (e) {
      console.error("refresh failed", e);
    }
  }, []);

  useEffect(() => {
    void refresh();
    let unlisten: (() => void) | undefined;
    void onEvent(EVENTS.fsChanged, () => {
      void refresh();
    }).then((f) => {
      unlisten = f;
    });
    return () => unlisten?.();
  }, [refresh]);

  // 图标懒加载（shell 图标 → base64，按路径缓存；失败不再重试）
  useEffect(() => {
    let cancelled = false;
    for (const item of items) {
      if (icons.has(item.path) || iconErrors.has(item.path)) continue;
      containerIcon(item.path)
        .then((b64) => {
          if (cancelled) return;
          if (!b64 || b64.length < 32) {
            setIconErrors((m) => new Map(m).set(item.path, "empty"));
            return;
          }
          setIcons((m) => new Map(m).set(item.path, b64));
        })
        .catch((e) => {
          if (cancelled) return;
          console.error("icon invoke failed", item.path, e);
          setIconErrors((m) => new Map(m).set(item.path, String(e)));
        });
    }
    return () => {
      cancelled = true;
    };
  }, [items, icons, iconErrors]);

  // 诊断滴答（S4 回归）：仅 ?diag=1（debug 构建自动携带）时每秒上报
  useEffect(() => {
    if (!shouldEnableDiag(window.location.search)) return;
    const id = window.setInterval(() => {
      void diagTick().catch(() => {});
    }, 1000);
    return () => window.clearInterval(id);
  }, []);

  // 桌面右键替换：钩子拦截桌面右键 → 在容器局部坐标弹出自定义菜单
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void onEvent<{ x: number; y: number }>(EVENTS.desktopMenuClick, (p) => {
      setMenu({ x: p.x, y: p.y, path: null });
      setGroupSubmenu(false);
    }).then((f) => {
      unlisten = f;
    });
    return () => unlisten?.();
  }, []);

  const indexOf = useMemo(() => {
    const m = new Map<string, number>();
    items.forEach((item, i) => m.set(item.path, i));
    return m;
  }, [items]);

  const posOf = useCallback(
    (item: ContainerItem): Pos => {
      const saved = positions.get(item.path);
      return resolvePos(saved, indexOf.get(item.path) ?? 0, DRAG);
    },
    [positions, indexOf],
  );

  const clearMenu = useCallback(() => {
    setMenu(null);
    setGroupSubmenu(false);
  }, []);

  const toLocal = useCallback((e: React.PointerEvent): Pos => {
    const rect = pageRef.current?.getBoundingClientRect();
    return { x: e.clientX - (rect?.left ?? 0), y: e.clientY - (rect?.top ?? 0) };
  }, []);

  const onPointerDown = useCallback(
    (e: React.PointerEvent, item?: ContainerItem) => {
      if (e.button !== 0) return;
      clearMenu();
      const p = toLocal(e);
      if (item) {
        const multi = e.ctrlKey || e.metaKey;
        setSelected((s) => {
          const ns = new Set(multi ? s : []);
          if (multi && s.has(item.path)) ns.delete(item.path);
          else ns.add(item.path);
          return ns;
        });
        // 真实起点 = 该项目的实际渲染位（已存布局或默认网格位）
        const origin = posOf(item);
        dragRef.current = {
          startX: p.x,
          startY: p.y,
          path: item.path,
          origin,
          cur: origin,
          moved: false,
        };
        (e.target as HTMLElement).setPointerCapture(e.pointerId);
      } else {
        // 空白区：开始框选
        setSelected(new Set());
        const b = { x0: p.x, y0: p.y, x1: p.x, y1: p.y };
        bandRef.current = b;
        setBand(b);
        (e.target as HTMLElement).setPointerCapture(e.pointerId);
      }
    },
    [clearMenu, toLocal, posOf],
  );

  const onPointerMove = useCallback(
    (e: React.PointerEvent) => {
      const p = toLocal(e);
      if (dragRef.current) {
        const d = dragRef.current;
        const dx = p.x - d.startX;
        const dy = p.y - d.startY;
        if (!d.moved && Math.hypot(dx, dy) > 4) d.moved = true;
        if (d.moved) {
          d.cur = { x: d.origin.x + dx, y: d.origin.y + dy };
          setDragPos(d.cur);
        }
      } else if (bandRef.current) {
        const b = { ...bandRef.current, x1: p.x, y1: p.y };
        bandRef.current = b;
        setBand(b);
      }
    },
    [toLocal],
  );

  const onPointerUp = useCallback(() => {
    if (dragRef.current) {
      const d = dragRef.current;
      if (d.moved) {
        // 用 ref 里的最终位置（状态异步，读 state 会滞后一帧导致落点偏移）
        const snapped = snapPos(d.cur, DRAG);
        const selectedItems = items.filter((it) => selected.has(it.path));
        const origins = selectedItems.map((it) => ({
          path: it.path,
          origin: posOf(it),
        }));
        const entries = dropEntries(origins, d.path, snapped.x, snapped.y);
        if (entries.length > 0) {
          // 本地立即回填全部移动项（含未持久化的选中项）
          setPositions((m) => {
            const nm = new Map(m);
            for (const en of entries) nm.set(en.path, { x: en.x, y: en.y });
            return nm;
          });
          void containerSaveLayout(entries);
        }
      }
      dragRef.current = null;
      setDragPos(null);
    } else if (bandRef.current) {
      // 框选结束：命中检测
      const b = bandRef.current;
      const bx0 = Math.min(b.x0, b.x1);
      const by0 = Math.min(b.y0, b.y1);
      const bx1 = Math.max(b.x0, b.x1);
      const by1 = Math.max(b.y0, b.y1);
      const hit = new Set<string>();
      items.forEach((item) => {
        const pos = posOf(item);
        if (
          pos.x + DRAG.cellW > bx0 &&
          pos.x < bx1 &&
          pos.y + DRAG.cellH > by0 &&
          pos.y < by1
        ) {
          hit.add(item.path);
        }
      });
      if (hit.size > 0) setSelected(hit);
      bandRef.current = null;
      setBand(null);
    }
  }, [items, selected, posOf]);

  const onContextMenu = useCallback(
    (e: React.MouseEvent, item?: ContainerItem) => {
      e.preventDefault();
      const p = toLocal(e as unknown as React.PointerEvent);
      setMenu({ x: p.x, y: p.y, path: item?.path ?? null });
      setGroupSubmenu(false);
    },
    [toLocal],
  );

  const onDoubleClick = useCallback((item: ContainerItem) => {
    void containerOpenItem(item.path);
  }, []);

  const doGroupAdd = useCallback(
    async (groupId: string, path: string) => {
      await groupAddItem(groupId, path);
      setGroups(await groupList());
      clearMenu();
    },
    [clearMenu],
  );

  const doGroupCreate = useCallback(async () => {
    const name = window.prompt("分组名称");
    if (!name) return;
    const gs = await groupCreate(name);
    setGroups(gs);
    if (menu?.path) {
      const g = gs.find((x) => x.name === name);
      if (g) await doGroupAdd(g.id, menu.path);
    }
  }, [menu, doGroupAdd]);

  return (
    <div
      ref={pageRef}
      className={styles.page}
      onPointerDown={(e) => {
        if (e.target === e.currentTarget) onPointerDown(e);
      }}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onContextMenu={(e) => {
        if (e.target === e.currentTarget) onContextMenu(e);
      }}
    >
      {items.map((item) => {
        const isDragged = dragRef.current?.path === item.path;
        const pos = isDragged && dragPos ? dragPos : posOf(item);
        const isSel = selected.has(item.path);
        return (
          <div
            key={item.path}
            className={`${styles.item} ${isSel ? styles.selected : ""}`}
            style={{ left: pos.x, top: pos.y, width: DRAG.cellW, height: DRAG.cellH }}
            onPointerDown={(e) => onPointerDown(e, item)}
            onContextMenu={(e) => onContextMenu(e, item)}
            onDoubleClick={() => onDoubleClick(item)}
            title={item.name}
          >
            <div className={styles.iconWrap}>
              {icons.get(item.path) ? (
                <img className={styles.icon} src={`data:image/png;base64,${icons.get(item.path)}`} alt="" draggable={false} />
              ) : (
                <div className={styles.iconPlaceholder}>
                  {iconErrors.has(item.path) ? "!" : item.name.slice(0, 1)}
                </div>
              )}
            </div>
            <div className={styles.label}>{item.name}</div>
          </div>
        );
      })}

      {band && (
        <div
          className={styles.band}
          style={{
            left: Math.min(band.x0, band.x1),
            top: Math.min(band.y0, band.y1),
            width: Math.abs(band.x1 - band.x0),
            height: Math.abs(band.y1 - band.y0),
          }}
        />
      )}

      {menu && (
        <div className={styles.menu} style={{ left: menu.x, top: menu.y }} onPointerDown={(e) => e.stopPropagation()}>
          {menu.path && (
            <>
              <button className={styles.menuItem} onClick={() => { void containerOpenItem(menu.path!); clearMenu(); }}>
                打开
              </button>
              <button className={styles.menuItem} onClick={() => { void containerRevealItem(menu.path!); clearMenu(); }}>
                在资源管理器中定位
              </button>
              <button
                className={styles.menuItem}
                onClick={() => setGroupSubmenu((v) => !v)}
              >
                移入分组 ▸
              </button>
              {groupSubmenu && (
                <div className={styles.submenu}>
                  {groups.map((g) => (
                    <button
                      key={g.id}
                      className={styles.menuItem}
                      onClick={() => void doGroupAdd(g.id, menu.path!)}
                    >
                      {g.name}
                    </button>
                  ))}
                  <button className={styles.menuItem} onClick={() => void doGroupCreate()}>
                    ＋ 新建分组…
                  </button>
                </div>
              )}
            </>
          )}
          {!menu.path && (
            <>
              <button className={styles.menuItem} onClick={() => void doGroupCreate()}>
                新建分组…
              </button>
              <button className={styles.menuItem} onClick={() => { void refresh(); clearMenu(); }}>
                刷新
              </button>
            </>
          )}
        </div>
      )}
    </div>
  );
}
