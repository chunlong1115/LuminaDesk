import { beforeEach, describe, expect, it } from "vitest";
import { useStats } from "./stats";

describe("stats 环形缓冲", () => {
  beforeEach(() => {
    useStats.setState({ history: [] });
  });

  it("最多保留 60 点", () => {
    for (let i = 0; i < 80; i++) {
      useStats.getState().push({ cpu: i, memUsedMb: 1, memTotalMb: 16 });
    }
    const h = useStats.getState().history;
    expect(h.length).toBe(60);
    expect(h[0]?.cpu).toBe(20);
    expect(h[59]?.cpu).toBe(79);
  });

  it("顺序追加", () => {
    useStats.getState().push({ cpu: 1, memUsedMb: 1, memTotalMb: 16 });
    useStats.getState().push({ cpu: 2, memUsedMb: 1, memTotalMb: 16 });
    expect(useStats.getState().history.map((s) => s.cpu)).toEqual([1, 2]);
  });
});
