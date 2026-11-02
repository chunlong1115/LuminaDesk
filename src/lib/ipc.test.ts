import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  EVENTS,
  hitTestSetRegions,
  onEvent,
  statsSubscribe,
  systemSetWidgetsAboveIcons,
  widgetsSetActive,
} from "./ipc";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const invokeMock = invoke as unknown as ReturnType<typeof vi.fn>;
const listenMock = listen as unknown as ReturnType<typeof vi.fn>;

describe("ipc 封装", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("statsSubscribe 订阅：命令名与参数映射正确", async () => {
    invokeMock.mockResolvedValue(2);
    await expect(statsSubscribe(true)).resolves.toBe(2);
    expect(invokeMock).toHaveBeenCalledWith("stats_subscribe", { enabled: true });
  });

  it("statsSubscribe 退订传 false", async () => {
    invokeMock.mockResolvedValue(0);
    await statsSubscribe(false);
    expect(invokeMock).toHaveBeenCalledWith("stats_subscribe", { enabled: false });
  });

  it("hitTestSetRegions 上报可点击区：命令名与参数映射正确", async () => {
    invokeMock.mockResolvedValue(undefined);
    const regions = [
      { x: 1200, y: 28, w: 200, h: 50 },
      { x: 0, y: -10, w: 30, h: 30 },
    ];
    await hitTestSetRegions(regions);
    expect(invokeMock).toHaveBeenCalledWith("hit_test_set_regions", { regions });
  });

  it("widgetsSetActive 激活/失活映射", async () => {
    invokeMock.mockResolvedValue(undefined);
    await widgetsSetActive(true);
    await widgetsSetActive(false);
    expect(invokeMock).toHaveBeenNthCalledWith(1, "widgets_set_active", { active: true });
    expect(invokeMock).toHaveBeenNthCalledWith(2, "widgets_set_active", { active: false });
  });

  it("systemSetWidgetsAboveIcons 开关：透传布尔并返回结果", async () => {
    invokeMock.mockResolvedValue(true);
    await expect(systemSetWidgetsAboveIcons(true)).resolves.toBe(true);
    expect(invokeMock).toHaveBeenCalledWith("system_set_widgets_above_icons", {
      enabled: true,
    });
  });

  it("onEvent 注册监听、透传 payload，并返回取消函数", async () => {
    const off = vi.fn();
    listenMock.mockResolvedValue(off);
    const cb = vi.fn();
    const unlisten = await onEvent<{ cpu_percent: number }>(EVENTS.statsSample, cb);
    expect(listenMock).toHaveBeenCalledWith(
      EVENTS.statsSample,
      expect.any(Function),
    );
    const handler = listenMock.mock.calls[0]![1] as (e: { payload: unknown }) => void;
    handler({ payload: { cpu_percent: 42 } });
    expect(cb).toHaveBeenCalledWith({ cpu_percent: 42 });
    unlisten();
    expect(off).toHaveBeenCalled();
  });
});
