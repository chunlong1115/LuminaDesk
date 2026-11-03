import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { clampNoteSize, _resetForTest, useNotes } from "./notes";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = invoke as unknown as ReturnType<typeof vi.fn>;

const seedNote = {
  id: "n1",
  x: 0,
  y: 0,
  w: 200,
  h: 160,
  text: "",
};

describe("clampNoteSize 缩放钳位", () => {
  it("低于下限兜底", () => {
    expect(clampNoteSize(10, 20)).toEqual({ w: 160, h: 120 });
  });

  it("常规值四舍五入", () => {
    expect(clampNoteSize(210.4, 180.6)).toEqual({ w: 210, h: 181 });
  });
});

describe("useNotes 便签状态", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    invokeMock.mockResolvedValue([]);
    useNotes.setState({ notes: [] });
    _resetForTest();
  });

  it("新增：id 唯一、默认原点阶梯偏移", () => {
    useNotes.getState().add();
    useNotes.getState().add();
    const notes = useNotes.getState().notes;
    expect(notes).toHaveLength(2);
    expect(notes[0]!.id).not.toBe(notes[1]!.id);
    expect(notes[1]!.x - notes[0]!.x).toBe(28);
    expect(notes[1]!.y - notes[0]!.y).toBe(28);
  });

  it("更新：合并 patch，不动其他字段", () => {
    useNotes.getState().add();
    const id = useNotes.getState().notes[0]!.id;
    useNotes.getState().update(id, { text: "hello", x: 300 });
    const n = useNotes.getState().notes[0]!;
    expect(n.text).toBe("hello");
    expect(n.x).toBe(300);
    expect(n.w).toBe(200);
  });

  it("删除：仅移除目标", () => {
    useNotes.getState().add();
    useNotes.getState().add();
    const [a, b] = useNotes.getState().notes;
    useNotes.getState().remove(a!.id);
    expect(useNotes.getState().notes.map((n) => n.id)).toEqual([b!.id]);
  });
});

describe("useNotes Rust 持久化接线", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    invokeMock.mockResolvedValue([]);
    useNotes.setState({ notes: [] });
    _resetForTest();
  });

  it("hydrate 从 Rust 加载便签列表", async () => {
    invokeMock.mockResolvedValue([seedNote]);
    useNotes.getState().hydrate();
    await vi.waitFor(() => {
      expect(useNotes.getState().notes).toEqual([seedNote]);
    });
    expect(invokeMock).toHaveBeenCalledWith("notes_list");
  });

  it("add 立即落盘新便签", async () => {
    useNotes.getState().add();
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("notes_upsert", {
        note: expect.objectContaining({ text: "", w: 200, h: 160 }),
      });
    });
  });

  it("update 去抖：300ms 内多次改动只落盘一次最终值", async () => {
    vi.useFakeTimers();
    useNotes.setState({ notes: [seedNote] });
    vi.clearAllMocks();
    useNotes.getState().update("n1", { text: "a" });
    useNotes.getState().update("n1", { text: "ab" });
    expect(invokeMock).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(300);
    await Promise.resolve();
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("notes_upsert", {
      note: expect.objectContaining({ id: "n1", text: "ab" }),
    });
    vi.useRealTimers();
  });

  it("remove 落盘并取消待去抖项（不再 upsert）", async () => {
    vi.useFakeTimers();
    useNotes.setState({ notes: [seedNote] });
    vi.clearAllMocks();
    useNotes.getState().update("n1", { text: "x" });
    useNotes.getState().remove("n1");
    await vi.advanceTimersByTimeAsync(500);
    await Promise.resolve();
    expect(invokeMock).toHaveBeenCalledWith("notes_remove", { id: "n1" });
    expect(invokeMock).not.toHaveBeenCalledWith("notes_upsert", expect.anything());
    vi.useRealTimers();
  });
});
