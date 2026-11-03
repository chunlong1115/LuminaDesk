import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { isValidTodoText, _resetForTest, useTodos } from "./todo";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = invoke as unknown as ReturnType<typeof vi.fn>;

describe("isValidTodoText 空文本校验", () => {
  it("空白拒绝、正常通过", () => {
    expect(isValidTodoText("")).toBe(false);
    expect(isValidTodoText("   ")).toBe(false);
    expect(isValidTodoText("买牛奶")).toBe(true);
  });
});

describe("useTodos 待办状态", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    invokeMock.mockResolvedValue([]);
    useTodos.setState({ todos: [] });
    _resetForTest();
  });

  it("新增：trim 后入库、id 唯一、默认未完成", () => {
    useTodos.getState().add(" 买牛奶 ");
    useTodos.getState().add("写周报");
    const todos = useTodos.getState().todos;
    expect(todos).toHaveLength(2);
    expect(todos[0]!.text).toBe("买牛奶");
    expect(todos[0]!.done).toBe(false);
    expect(todos[0]!.id).not.toBe(todos[1]!.id);
  });

  it("勾选切换 done", () => {
    useTodos.getState().add("a");
    const id = useTodos.getState().todos[0]!.id;
    useTodos.getState().toggle(id);
    expect(useTodos.getState().todos[0]!.done).toBe(true);
    useTodos.getState().toggle(id);
    expect(useTodos.getState().todos[0]!.done).toBe(false);
  });

  it("删除仅移除目标", () => {
    useTodos.getState().add("a");
    useTodos.getState().add("b");
    const [a, b] = useTodos.getState().todos;
    useTodos.getState().remove(a!.id);
    expect(useTodos.getState().todos.map((t) => t.id)).toEqual([b!.id]);
  });
});

describe("useTodos Rust 持久化接线", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    invokeMock.mockResolvedValue([]);
    useTodos.setState({ todos: [] });
    _resetForTest();
  });

  it("hydrate 从 Rust 加载待办列表", async () => {
    const rustList = [{ id: "t1", text: "买牛奶", done: true }];
    invokeMock.mockResolvedValue(rustList);
    useTodos.getState().hydrate();
    await vi.waitFor(() => {
      expect(useTodos.getState().todos).toEqual(rustList);
    });
    expect(invokeMock).toHaveBeenCalledWith("todos_list");
  });

  it("add 落盘新待办（trim 后文本）", async () => {
    useTodos.getState().add(" 买牛奶 ");
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("todos_upsert", {
        todo: expect.objectContaining({ text: "买牛奶", done: false }),
      });
    });
  });

  it("toggle 落盘勾选态", async () => {
    useTodos.setState({ todos: [{ id: "t1", text: "a", done: false }] });
    vi.clearAllMocks();
    useTodos.getState().toggle("t1");
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("todos_upsert", {
        todo: expect.objectContaining({ id: "t1", done: true }),
      });
    });
  });

  it("remove 落盘删除", async () => {
    useTodos.setState({ todos: [{ id: "t1", text: "a", done: false }] });
    vi.clearAllMocks();
    useTodos.getState().remove("t1");
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("todos_remove", { id: "t1" });
    });
  });
});
