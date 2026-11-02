import { describe, expect, it } from "vitest";
import { clampPct, formatClock, formatDate, formatMb, msToNextSecond } from "./format";

describe("clampPct 钳位 0..=100", () => {
  it("越界钳位", () => {
    expect(clampPct(150)).toBe(100);
    expect(clampPct(-5)).toBe(0);
    expect(clampPct(42.7)).toBe(42.7);
  });

  it("NaN/Infinity 归一为 0", () => {
    expect(clampPct(NaN)).toBe(0);
    expect(clampPct(Infinity)).toBe(0);
    expect(clampPct(-Infinity)).toBe(0);
  });
});

describe("formatMb 内存格式化", () => {
  it("<1GB 显示整数 MB", () => {
    expect(formatMb(512)).toBe("512 MB");
    expect(formatMb(1023)).toBe("1023 MB");
  });

  it("≥1GB 显示 1 位小数 GB", () => {
    expect(formatMb(1024)).toBe("1.0 GB");
    expect(formatMb(16384)).toBe("16.0 GB");
  });
});

describe("formatClock 补零", () => {
  it("个位时分秒补零", () => {
    expect(formatClock(new Date(2026, 0, 1, 8, 5, 9))).toBe("08:05:09");
  });

  it("双位不补", () => {
    expect(formatClock(new Date(2026, 0, 1, 23, 59, 58))).toBe("23:59:58");
  });
});

describe("msToNextSecond 秒沿对齐", () => {
  it("毫秒 250 → 750", () => {
    expect(msToNextSecond(new Date(2026, 0, 1, 0, 0, 0, 250))).toBe(750);
  });

  it("毫秒 0 → 1000（刚过整秒，等满下一秒）", () => {
    expect(msToNextSecond(new Date(2026, 0, 1, 0, 0, 0, 0))).toBe(1000);
  });

  it("毫秒 999 → 1", () => {
    expect(msToNextSecond(new Date(2026, 0, 1, 0, 0, 0, 999))).toBe(1);
  });
});

describe("formatDate 中文日期", () => {
  it("2026-11-02 为周一", () => {
    expect(formatDate(new Date(2026, 10, 2))).toBe("2026年11月2日 周一");
  });
});
