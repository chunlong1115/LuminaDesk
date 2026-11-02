import { beforeEach, describe, expect, it } from "vitest";
import { clampNoteSize, useNotes } from "./notes";

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
    useNotes.setState({ notes: [] });
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
