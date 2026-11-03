import { describe, expect, it } from "vitest";
import { newWidgetId, nextWidgetPos, parsePos, WIDGET_KIND_LABELS } from "./widgetPanel";

describe("WIDGET_KIND_LABELS 类型覆盖", () => {
  it("五种组件齐备", () => {
    expect(Object.keys(WIDGET_KIND_LABELS).sort()).toEqual([
      "clock",
      "cpuMem",
      "date",
      "notes",
      "todo",
    ]);
  });
});

describe("nextWidgetPos 新增位置", () => {
  it("空列表从原点开始", () => {
    expect(nextWidgetPos([])).toEqual({ x: 100, y: 100 });
  });

  it("按数量阶梯偏移，每 6 个换行", () => {
    const list = Array.from({ length: 7 }, (_, i) => ({
      id: `w${i}`,
      kind: "clock" as const,
      x: 0,
      y: 0,
    }));
    expect(nextWidgetPos(list)).toEqual({ x: 160, y: 160 });
  });
});

describe("newWidgetId 唯一性", () => {
  it("含 kind 前缀且连续调用不冲突", () => {
    const a = newWidgetId("clock");
    const b = newWidgetId("clock");
    expect(a.startsWith("clock-")).toBe(true);
    expect(a).not.toBe(b);
  });
});

describe("parsePos 位置解析", () => {
  it("整数通过（含负数）", () => {
    expect(parsePos("300", 10)).toBe(300);
    expect(parsePos("-20", 10)).toBe(-20);
  });

  it("空白/非法/小数回退原值", () => {
    expect(parsePos("", 10)).toBe(10);
    expect(parsePos("  ", 10)).toBe(10);
    expect(parsePos("abc", 10)).toBe(10);
    expect(parsePos("3.5", 10)).toBe(10);
  });
});
