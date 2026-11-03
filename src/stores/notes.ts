import { create } from "zustand";
import { notesList, notesRemove, notesUpsert, type NoteItem } from "../lib/ipc";

export type { NoteItem };

/** 便签尺寸下限（拖拽缩放钳位） */
export const MIN_NOTE_W = 160;
export const MIN_NOTE_H = 120;

/** 新便签默认原点（容器逻辑坐标；后续组件管理页可配） */
export const NOTES_ORIGIN = { x: 1020, y: 340 };

/** 缩放钳位纯函数：向下取整并兜住下限 */
export function clampNoteSize(w: number, h: number): { w: number; h: number } {
  return {
    w: Math.max(MIN_NOTE_W, Math.round(w)),
    h: Math.max(MIN_NOTE_H, Math.round(h)),
  };
}

let nextId = 0;

/** 生成唯一便签 id（模块级计数，避免同毫秒碰撞） */
export function newNoteId(): string {
  nextId += 1;
  return `note-${Date.now()}-${nextId}`;
}

// ---- Rust 持久化接线（widgets.json，单窗口编辑不广播） ----

/** 持久化写链：所有 IPC 写按调用序串行——删除晚于在途 upsert 也不会复活 */
let writeChain: Promise<void> = Promise.resolve();

function enqueue(task: () => Promise<void>): void {
  writeChain = writeChain.then(task).catch((e) => {
    console.error("notes persist failed", e);
  });
}

/** 拖拽/打字高频 update → 去抖后批量落盘 */
const UPSERT_DEBOUNCE_MS = 300;
let flushTimer: ReturnType<typeof setTimeout> | null = null;
const pendingIds = new Set<string>();
let hydrated = false;

/** 仅测试用：重置持久化接线状态 */
export function _resetForTest(): void {
  writeChain = Promise.resolve();
  if (flushTimer) clearTimeout(flushTimer);
  flushTimer = null;
  pendingIds.clear();
  hydrated = false;
}

function flushPending(): void {
  flushTimer = null;
  const ids = [...pendingIds];
  pendingIds.clear();
  enqueue(async () => {
    for (const id of ids) {
      const n = useNotes.getState().notes.find((x) => x.id === id);
      if (n) await notesUpsert(n); // 执行时已删则跳过（删除在写链后序）
    }
  });
}

interface NotesState {
  notes: NoteItem[];
  add: () => void;
  update: (id: string, patch: Partial<Omit<NoteItem, "id">>) => void;
  remove: (id: string) => void;
  /** 从 Rust 加载便签（幂等） */
  hydrate: () => void;
}

/** 便签状态（Rust widgets.json 持久化；本地即时生效，落盘走写链） */
export const useNotes = create<NotesState>((set) => ({
  notes: [],
  add: () => {
    const id = newNoteId();
    const len = useNotes.getState().notes.length;
    const note: NoteItem = {
      id,
      text: "",
      x: NOTES_ORIGIN.x + len * 28,
      y: NOTES_ORIGIN.y + len * 28,
      w: 200,
      h: 160,
    };
    set((s) => ({ notes: [...s.notes, note] }));
    enqueue(async () => {
      await notesUpsert(note);
    });
  },
  update: (id, patch) => {
    set((s) => ({
      notes: s.notes.map((n) => (n.id === id ? { ...n, ...patch } : n)),
    }));
    pendingIds.add(id);
    if (!flushTimer) flushTimer = setTimeout(flushPending, UPSERT_DEBOUNCE_MS);
  },
  remove: (id) => {
    set((s) => ({ notes: s.notes.filter((n) => n.id !== id) }));
    pendingIds.delete(id);
    enqueue(async () => {
      await notesRemove(id);
    });
  },
  hydrate: () => {
    if (hydrated) return;
    hydrated = true;
    void (async () => {
      try {
        const list = await notesList();
        set({ notes: list });
      } catch (e) {
        console.error("notes hydrate failed", e);
        hydrated = false;
      }
    })();
  },
}));
