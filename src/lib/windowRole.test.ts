import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { isWidgetsRole, setActiveIfWidgets } from "./windowRole";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = invoke as unknown as ReturnType<typeof vi.fn>;

describe("isWidgetsRole", () => {
  it("?role=widgets 为浮层窗口", () => {
    expect(isWidgetsRole("?role=widgets")).toBe(true);
  });

  it("无 role/其他 role 为否", () => {
    expect(isWidgetsRole("")).toBe(false);
    expect(isWidgetsRole("?role=control")).toBe(false);
    expect(isWidgetsRole("?foo=1")).toBe(false);
  });
});

describe("setActiveIfWidgets", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("浮层窗口聚焦/失焦 → 转发激活/恢复", () => {
    invokeMock.mockResolvedValue(undefined);
    setActiveIfWidgets("?role=widgets", true);
    setActiveIfWidgets("?role=widgets", false);
    expect(invokeMock).toHaveBeenNthCalledWith(1, "widgets_set_active", { active: true });
    expect(invokeMock).toHaveBeenNthCalledWith(2, "widgets_set_active", { active: false });
  });

  it("非浮层窗口忽略", () => {
    setActiveIfWidgets("", true);
    setActiveIfWidgets("?role=control", false);
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
