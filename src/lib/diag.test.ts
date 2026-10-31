import { describe, expect, it } from "vitest";
import { shouldEnableDiag } from "./diag";

describe("诊断模式开关", () => {
  it("diag=1 启用", () => {
    expect(shouldEnableDiag("?role=container&diag=1")).toBe(true);
  });

  it("无参数不启用", () => {
    expect(shouldEnableDiag("?role=container")).toBe(false);
    expect(shouldEnableDiag("")).toBe(false);
  });

  it("diag=0 或非法值不启用", () => {
    expect(shouldEnableDiag("?diag=0")).toBe(false);
    expect(shouldEnableDiag("?diag=yes")).toBe(false);
  });
});
