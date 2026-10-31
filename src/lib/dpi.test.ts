import { describe, expect, it } from "vitest";
import { clamp, logicalToPhysical, physicalToLogical } from "./dpi";

describe("dpi 换算", () => {
  it("125% 缩放：逻辑↔物理", () => {
    expect(logicalToPhysical(100, 1.25)).toBe(125);
    expect(physicalToLogical(125, 1.25)).toBe(100);
  });

  it("150% 缩放：逻辑↔物理", () => {
    expect(logicalToPhysical(100, 1.5)).toBe(150);
    expect(physicalToLogical(150, 1.5)).toBe(100);
  });

  it("clamp 钳位", () => {
    expect(clamp(150, 0, 100)).toBe(100);
    expect(clamp(-5, 0, 100)).toBe(0);
    expect(clamp(50, 0, 100)).toBe(50);
  });
});
