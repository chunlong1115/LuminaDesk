import { describe, expect, it } from "vitest";
import { sparkPoints } from "./Sparkline";

describe("sparkPoints 坐标映射", () => {
  it("不足 2 点返回空串（组件不渲染）", () => {
    expect(sparkPoints([], 100, 30)).toBe("");
    expect(sparkPoints([5], 100, 30)).toBe("");
  });

  it("首点 x=0，末点 x=w（均布满宽）", () => {
    // max=3：v=1 → y=20，v=3 贴顶 y=0
    const pts = sparkPoints([1, 2, 3], 100, 30).split(" ");
    expect(pts[0]).toBe("0,20");
    expect(pts[2]).toBe("100,0");
  });

  it("最大值贴顶 y=0", () => {
    const pts = sparkPoints([0, 100], 100, 30).split(" ");
    expect(pts[0]).toBe("0,30");
    expect(pts[1]).toBe("100,0");
  });

  it("全零数据不除零", () => {
    const pts = sparkPoints([0, 0, 0], 100, 30).split(" ");
    expect(pts).toEqual(["0,30", "50,30", "100,30"]);
  });
});
