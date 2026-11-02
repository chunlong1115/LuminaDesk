import { create } from "zustand";

export interface TodoItem {
  id: string;
  text: string;
  done: boolean;
}

let nextId = 0;

/** 生成唯一待办 id */
export function newTodoId(): string {
  nextId += 1;
  return `todo-${Date.now()}-${nextId}`;
}

/** 空文本校验（添加前由组件调用；store 不静默丢弃） */
export function isValidTodoText(text: string): boolean {
  return text.trim().length > 0;
}

interface TodoState {
  todos: TodoItem[];
  add: (text: string) => void;
  toggle: (id: string) => void;
  remove: (id: string) => void;
}

/** 待办状态（阶段 3 前端内存版；持久化 CRUD 待 Rust 待办命令落地后接入） */
export const useTodos = create<TodoState>((set) => ({
  todos: [],
  add: (text) =>
    set((s) => ({
      todos: [...s.todos, { id: newTodoId(), text: text.trim(), done: false }],
    })),
  toggle: (id) =>
    set((s) => ({
      todos: s.todos.map((t) => (t.id === id ? { ...t, done: !t.done } : t)),
    })),
  remove: (id) => set((s) => ({ todos: s.todos.filter((t) => t.id !== id) })),
}));
