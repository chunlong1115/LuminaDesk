import { describe, expect, it } from "vitest";
import { dropEntries, gridPosOf, resolvePos, snapPos, type DragParams } from "./dragMath";

const P: DragParams = {
  cellW: 104,
  cellH: 96,
  cols: 5,
  originX: 16,
  originY: 16,
  snap: 8,
};

describe("dragMath 网格", () => {
  it("默认网格位按索引计算", () => {
    expect(gridPosOf(0, P)).toEqual({ x: 16, y: 16 });
    expect(gridPosOf(5, P)).toEqual({ x: 16, y: 112 });
    expect(gridPosOf(7, P)).toEqual({ x: 224, y: 112 });
  });

  it("已存布局优先于网格", () => {
    expect(resolvePos({ x: 99, y: 88 }, 0, P)).toEqual({ x: 99, y: 88 });
    expect(resolvePos(undefined, 3, P)).toEqual(gridPosOf(3, P));
  });

  it("吸附 8px 步进", () => {
    expect(snapPos({ x: 103, y: 100 }, P)).toEqual({ x: 104, y: 104 }); // 12.5 四舍五入到 13
    expect(snapPos({ x: 104, y: 96 }, P)).toEqual({ x: 104, y: 96 });
    expect(snapPos({ x: 99, y: 97 }, P)).toEqual({ x: 96, y: 96 });
  });
});

describe("dragMath 落点", () => {
  it("被拖项落在吸附点，其余项保持相对偏移", () => {
    const entries = dropEntries(
      [
        { path: "a", origin: { x: 100, y: 100 } },
        { path: "b", origin: { x: 300, y: 200 } },
      ],
      "a",
      148,
      140,
    );
    expect(entries).toEqual([
      { path: "a", x: 148, y: 140 },
      { path: "b", x: 348, y: 240 },
    ]);
  });

  it("被拖项不在选中集时返回空", () => {
    expect(dropEntries([{ path: "a", origin: { x: 0, y: 0 } }], "zzz", 10, 10)).toEqual([]);
  });

  it("未存盘项以其网格位为基准（不再跳位到索引 0）", () => {
    const entries = dropEntries(
      [
        { path: "a", origin: gridPosOf(3, P) }, // 无布局时的默认位
        { path: "b", origin: gridPosOf(4, P) },
      ],
      "a",
      400,
      100,
    );
    expect(entries[0]).toEqual({ path: "a", x: 400, y: 100 });
    // b 保持与 a 的相对偏移（原差一格）
    expect(entries[1]?.x).toBe(400 + P.cellW);
  });
});
