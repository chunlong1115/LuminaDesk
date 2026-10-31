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
  type LayoutEntry,
} from "../../lib/ipc";
import { diagTick, shouldEnableDiag } from "../../lib/diag";
import styles from "./ContainerPage.module.css";

const CELL_W = 104;
const CELL_H = 96;
const GRID_ORIGIN_X = 16;
const GRID_ORIGIN_Y = 16;
const COLS = 5;

interface Pos {
  x: number;
  y: number;
}

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
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [groups, setGroups] = useState<GroupEntry[]>([]);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [groupSubmenu, setGroupSubmenu] = useState(false);

  const dragRef = useRef<{
    startX: number;
    startY: number;
    path: string;
    origin: Pos;
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

  // 图标懒加载（shell 图标 → base64，按路径缓存）
  const [iconErrors, setIconErrors] = useState<Map<string, string>>(new Map());
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

  const posOf = useCallback(
    (item: ContainerItem, index: number): Pos => {
      const saved = positions.get(item.path);
      if (saved) return saved;
      // 未持久化过：默认网格位（按列表序）
      return {
        x: GRID_ORIGIN_X + (index % COLS) * CELL_W,
        y: GRID_ORIGIN_Y + Math.floor(index / COLS) * CELL_H,
      };
    },
    [positions],
  );

  const clearMenu = useCallback(() => {
    setMenu(null);
    setGroupSubmenu(false);
  }, []);

  // 指针事件（统一处理点选/拖拽/框选，坐标相对窗口）
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
        dragRef.current = { startX: p.x, startY: p.y, path: item.path, origin: posOf(item, 0), moved: false };
        (e.target as HTMLElement).setPointerCapture(e.pointerId);
      } else {
        // 空白区：开始框选
        setSelected(new Set());
        bandRef.current = { x0: p.x, y0: p.y, x1: p.x, y1: p.y };
        setBand(bandRef.current);
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
          setDragPos({ x: d.origin.x + dx, y: d.origin.y + dy });
        }
      } else if (bandRef.current) {
        const b = { ...bandRef.current, x1: p.x, y1: p.y };
        bandRef.current = b;
        setBand(b);
      }
    },
    [toLocal],
  );

  const onPointerUp = useCallback(
    (e: React.PointerEvent) => {
      const p = toLocal(e);
      if (dragRef.current) {
        const d = dragRef.current;
        if (d.moved && dragPos) {
          const nx = Math.round(dragPos.x / 8) * 8;
          const ny = Math.round(dragPos.y / 8) * 8;
          setPositions((m) => new Map(m).set(d.path, { x: nx, y: ny }));
          // 批量落盘：所有选中项一起移动（保持相对偏移）
          const entries: LayoutEntry[] = [];
          for (const sp of selected) {
            const cur = positions.get(sp) ?? d.origin;
            entries.push({
              path: sp,
              x: nx + (cur.x - d.origin.x),
              y: ny + (cur.y - d.origin.y),
            });
          }
          void containerSaveLayout(entries);
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
        items.forEach((item, i) => {
          const pos = posOf(item, i);
          if (
            pos.x + CELL_W > bx0 &&
            pos.x < bx1 &&
            pos.y + CELL_H > by0 &&
            pos.y < by1
          ) {
            hit.add(item.path);
          }
        });
        if (hit.size > 0) setSelected(hit);
        bandRef.current = null;
        setBand(null);
      }
      void p;
    },
    [toLocal, dragPos, selected, positions, items, posOf],
  );

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

  // 诊断滴答（S4 回归）：仅 ?diag=1（debug 构建自动携带）时每秒上报
  useEffect(() => {
    if (!shouldEnableDiag(window.location.search)) return;
    const id = window.setInterval(() => {
      void diagTick().catch(() => {});
    }, 1000);
    return () => window.clearInterval(id);
  }, []);

  const visibleItems = useMemo(() => items, [items]);

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
      {visibleItems.map((item, i) => {
        const pos = dragRef.current?.path === item.path && dragPos ? dragPos : posOf(item, i);
        const isSel = selected.has(item.path);
        return (
          <div
            key={item.path}
            className={`${styles.item} ${isSel ? styles.selected : ""}`}
            style={{ left: pos.x, top: pos.y, width: CELL_W, height: CELL_H }}
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
            <button className={styles.menuItem} onClick={() => void doGroupCreate()}>
              新建分组…
            </button>
          )}
        </div>
      )}
    </div>
  );
}
