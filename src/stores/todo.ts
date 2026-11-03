import { create } from "zustand";
import { todosList, todosRemove, todosUpsert, type TodoItem } from "../lib/ipc";

export type { TodoItem };

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

// ---- Rust 持久化接线（widgets.json，单窗口编辑不广播） ----

/** 持久化写链：所有 IPC 写按调用序串行 */
let writeChain: Promise<void> = Promise.resolve();

function enqueue(task: () => Promise<void>): void {
  writeChain = writeChain.then(task).catch((e) => {
    console.error("todos persist failed", e);
  });
}

let hydrated = false;

/** 仅测试用：重置持久化接线状态 */
export function _resetForTest(): void {
  writeChain = Promise.resolve();
  hydrated = false;
}

interface TodoState {
  todos: TodoItem[];
  add: (text: string) => void;
  toggle: (id: string) => void;
  remove: (id: string) => void;
  /** 从 Rust 加载待办（幂等） */
  hydrate: () => void;
}

/** 待办状态（Rust widgets.json 持久化；增删勾选低频，直接落盘） */
export const useTodos = create<TodoState>((set) => ({
  todos: [],
  add: (text) => {
    const todo: TodoItem = { id: newTodoId(), text: text.trim(), done: false };
    set((s) => ({ todos: [...s.todos, todo] }));
    enqueue(async () => {
      await todosUpsert(todo);
    });
  },
  toggle: (id) => {
    set((s) => ({
      todos: s.todos.map((t) => (t.id === id ? { ...t, done: !t.done } : t)),
    }));
    const t = useTodos.getState().todos.find((x) => x.id === id);
    if (t) {
      enqueue(async () => {
        await todosUpsert(t);
      });
    }
  },
  remove: (id) => {
    set((s) => ({ todos: s.todos.filter((t) => t.id !== id) }));
    enqueue(async () => {
      await todosRemove(id);
    });
  },
  hydrate: () => {
    if (hydrated) return;
    hydrated = true;
    void (async () => {
      try {
        const list = await todosList();
        set({ todos: list });
      } catch (e) {
        console.error("todos hydrate failed", e);
        hydrated = false;
      }
    })();
  },
}));
