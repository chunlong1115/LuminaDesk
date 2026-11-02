import { beforeEach, describe, expect, it } from "vitest";
import { isValidTodoText, useTodos } from "./todo";

describe("isValidTodoText 空文本校验", () => {
  it("空白拒绝、正常通过", () => {
    expect(isValidTodoText("")).toBe(false);
    expect(isValidTodoText("   ")).toBe(false);
    expect(isValidTodoText("买牛奶")).toBe(true);
  });
});

describe("useTodos 待办状态", () => {
  beforeEach(() => {
    useTodos.setState({ todos: [] });
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
