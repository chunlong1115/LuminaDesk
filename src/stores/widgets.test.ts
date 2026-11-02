import { beforeEach, describe, expect, it } from "vitest";
import { DEFAULT_WIDGETS, useWidgets } from "./widgets";

describe("DEFAULT_WIDGETS 默认配置", () => {
  it("五组件齐备且 id 唯一", () => {
    const kinds = DEFAULT_WIDGETS.map((w) => w.kind).sort();
    expect(kinds).toEqual(["clock", "cpuMem", "date", "notes", "todo"]);
    expect(new Set(DEFAULT_WIDGETS.map((w) => w.id)).size).toBe(5);
  });

  it("坐标在容器逻辑范围内", () => {
    for (const w of DEFAULT_WIDGETS) {
      expect(w.x).toBeGreaterThanOrEqual(0);
      expect(w.y).toBeGreaterThanOrEqual(0);
    }
  });
});

describe("useWidgets 配置状态", () => {
  beforeEach(() => {
    useWidgets.setState({ widgets: DEFAULT_WIDGETS });
  });

  it("setWidgets 整体替换", () => {
    useWidgets.getState().setWidgets([{ id: "only", kind: "clock", x: 0, y: 0 }]);
    expect(useWidgets.getState().widgets).toEqual([
      { id: "only", kind: "clock", x: 0, y: 0 },
    ]);
  });
});
