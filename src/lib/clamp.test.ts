import { describe, expect, it } from "vitest";
import { clampPosition } from "./clamp";

describe("clampPosition", () => {
  it("视口内位置保持不变", () => {
    expect(clampPosition(1200, 340, 264, 320, 1536, 864)).toEqual({ x: 1200, y: 340 });
  });

  it("超出右/下缘 → 收回视口（贴边）", () => {
    expect(clampPosition(1400, 700, 264, 320, 1536, 864)).toEqual({ x: 1272, y: 544 });
  });

  it("负坐标 → 归零", () => {
    expect(clampPosition(-40, -8, 200, 160, 1536, 864)).toEqual({ x: 0, y: 0 });
  });

  it("元素大于视口 → 贴齐左上", () => {
    expect(clampPosition(300, 200, 2000, 1000, 1536, 864)).toEqual({ x: 0, y: 0 });
  });

  it("非整数坐标四舍五入后夹取", () => {
    expect(clampPosition(1520.6, 340.4, 264, 320, 1536, 864)).toEqual({ x: 1272, y: 340 });
  });
});
