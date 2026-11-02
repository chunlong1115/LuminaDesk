import { create } from "zustand";

export interface NoteItem {
  id: string;
  text: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

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

interface NotesState {
  notes: NoteItem[];
  add: () => void;
  update: (id: string, patch: Partial<Omit<NoteItem, "id">>) => void;
  remove: (id: string) => void;
}

/** 便签状态（阶段 3 前端内存版；持久化 CRUD 待 Rust 便签命令落地后接入） */
export const useNotes = create<NotesState>((set) => ({
  notes: [],
  add: () =>
    set((s) => ({
      notes: [
        ...s.notes,
        {
          id: newNoteId(),
          text: "",
          x: NOTES_ORIGIN.x + s.notes.length * 28,
          y: NOTES_ORIGIN.y + s.notes.length * 28,
          w: 200,
          h: 160,
        },
      ],
    })),
  update: (id, patch) =>
    set((s) => ({
      notes: s.notes.map((n) => (n.id === id ? { ...n, ...patch } : n)),
    })),
  remove: (id) => set((s) => ({ notes: s.notes.filter((n) => n.id !== id) })),
}));
