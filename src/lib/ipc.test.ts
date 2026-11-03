import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  EVENTS,
  hitTestSetRegions,
  notesList,
  notesRemove,
  notesUpsert,
  onEvent,
  statsSubscribe,
  systemSetWidgetsAboveIcons,
  todosList,
  todosRemove,
  todosUpsert,
  widgetsList,
  widgetsRemove,
  widgetsSetActive,
  widgetsUpsert,
} from "./ipc";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const invokeMock = invoke as unknown as ReturnType<typeof vi.fn>;
const listenMock = listen as unknown as ReturnType<typeof vi.fn>;

describe("ipc 封装", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("statsSubscribe 订阅：命令名与参数映射正确", async () => {
    invokeMock.mockResolvedValue(2);
    await expect(statsSubscribe(true)).resolves.toBe(2);
    expect(invokeMock).toHaveBeenCalledWith("stats_subscribe", { enabled: true });
  });

  it("statsSubscribe 退订传 false", async () => {
    invokeMock.mockResolvedValue(0);
    await statsSubscribe(false);
    expect(invokeMock).toHaveBeenCalledWith("stats_subscribe", { enabled: false });
  });

  it("hitTestSetRegions 上报可点击区：命令名与参数映射正确", async () => {
    invokeMock.mockResolvedValue(undefined);
    const regions = [
      { x: 1200, y: 28, w: 200, h: 50 },
      { x: 0, y: -10, w: 30, h: 30 },
    ];
    await hitTestSetRegions(regions);
    expect(invokeMock).toHaveBeenCalledWith("hit_test_set_regions", { regions });
  });

  it("widgetsSetActive 激活/失活映射", async () => {
    invokeMock.mockResolvedValue(undefined);
    await widgetsSetActive(true);
    await widgetsSetActive(false);
    expect(invokeMock).toHaveBeenNthCalledWith(1, "widgets_set_active", { active: true });
    expect(invokeMock).toHaveBeenNthCalledWith(2, "widgets_set_active", { active: false });
  });

  it("systemSetWidgetsAboveIcons 开关：透传布尔并返回结果", async () => {
    invokeMock.mockResolvedValue(true);
    await expect(systemSetWidgetsAboveIcons(true)).resolves.toBe(true);
    expect(invokeMock).toHaveBeenCalledWith("system_set_widgets_above_icons", {
      enabled: true,
    });
  });

  it("widgets 三命令映射（list/upsert/remove）", async () => {
    invokeMock.mockResolvedValue([]);
    await widgetsList();
    await widgetsUpsert({ id: "clock", kind: "clock", x: 1, y: 2 });
    await widgetsRemove("clock");
    expect(invokeMock).toHaveBeenNthCalledWith(1, "widgets_list");
    expect(invokeMock).toHaveBeenNthCalledWith(2, "widgets_upsert", {
      config: { id: "clock", kind: "clock", x: 1, y: 2 },
    });
    expect(invokeMock).toHaveBeenNthCalledWith(3, "widgets_remove", { id: "clock" });
  });

  it("notes 三命令映射（list/upsert/remove）", async () => {
    invokeMock.mockResolvedValue([]);
    const note = { id: "n1", x: 0, y: 0, w: 200, h: 160, text: "hi" };
    await notesList();
    await notesUpsert(note);
    await notesRemove("n1");
    expect(invokeMock).toHaveBeenNthCalledWith(1, "notes_list");
    expect(invokeMock).toHaveBeenNthCalledWith(2, "notes_upsert", { note });
    expect(invokeMock).toHaveBeenNthCalledWith(3, "notes_remove", { id: "n1" });
  });

  it("todos 三命令映射（list/upsert/remove）", async () => {
    invokeMock.mockResolvedValue([]);
    const todo = { id: "t1", text: "x", done: false };
    await todosList();
    await todosUpsert(todo);
    await todosRemove("t1");
    expect(invokeMock).toHaveBeenNthCalledWith(1, "todos_list");
    expect(invokeMock).toHaveBeenNthCalledWith(2, "todos_upsert", { todo });
    expect(invokeMock).toHaveBeenNthCalledWith(3, "todos_remove", { id: "t1" });
  });

  it("onEvent 注册监听、透传 payload，并返回取消函数", async () => {
    const off = vi.fn();
    listenMock.mockResolvedValue(off);
    const cb = vi.fn();
    const unlisten = await onEvent<{ cpu_percent: number }>(EVENTS.statsSample, cb);
    expect(listenMock).toHaveBeenCalledWith(
      EVENTS.statsSample,
      expect.any(Function),
    );
    const handler = listenMock.mock.calls[0]![1] as (e: { payload: unknown }) => void;
    handler({ payload: { cpu_percent: 42 } });
    expect(cb).toHaveBeenCalledWith({ cpu_percent: 42 });
    unlisten();
    expect(off).toHaveBeenCalled();
  });
});
