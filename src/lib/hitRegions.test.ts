import { describe, expect, it } from "vitest";
import { regionsFromRects, sameRegions } from "./hitRegions";

describe("regionsFromRects", () => {
  it("dpr=2：左/上向下取整、右/下向上取整", () => {
    const rects = [{ x: 10.3, y: 20.7, width: 100, height: 50.2 }];
    // x=floor(20.6)=20, y=floor(41.4)=41, right=ceil(220.6)=221, bottom=ceil(141.8)=142
    expect(regionsFromRects(rects, 2)).toEqual([{ x: 20, y: 41, w: 201, h: 101 }]);
  });

  it("dpr=1.25（125% 缩放）：物理像素换算", () => {
    const rects = [{ x: 1200, y: 28, width: 200, height: 50 }];
    // x=1500, y=35, right=ceil(1750)=1750 → w=250, bottom=ceil(97.5)=98 → h=63
    expect(regionsFromRects(rects, 1.25)).toEqual([{ x: 1500, y: 35, w: 250, h: 63 }]);
  });

  it("空列表", () => {
    expect(regionsFromRects([], 1.5)).toEqual([]);
  });
});

describe("sameRegions", () => {
  const a = [{ x: 1, y: 2, w: 3, h: 4 }];

  it("相同列表 true（含空）", () => {
    expect(sameRegions(a, [{ x: 1, y: 2, w: 3, h: 4 }])).toBe(true);
    expect(sameRegions([], [])).toBe(true);
  });

  it("长度不同 false", () => {
    expect(sameRegions(a, [])).toBe(false);
    expect(sameRegions(a, [...a, { x: 0, y: 0, w: 1, h: 1 }])).toBe(false);
  });

  it("任一字段不同 false", () => {
    expect(sameRegions(a, [{ x: 1, y: 2, w: 3, h: 5 }])).toBe(false);
    expect(sameRegions(a, [{ x: 2, y: 2, w: 3, h: 4 }])).toBe(false);
  });
});
