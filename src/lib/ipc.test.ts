import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { EVENTS, onEvent, statsSubscribe } from "./ipc";

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

  it("onEvent 注册监听、透传 payload，并返回取消函数", async () => {
    const off = vi.fn();
    listenMock.mockResolvedValue(off);
    const cb = vi.fn();
    const unlisten = await onEvent<{ cpu_percent: number }>(EVENTS.statsSample, cb);
    expect(listenMock).toHaveBeenCalledWith(
      EVENTS.statsSample,
      expect.any(Function),
    );
    const handler = listenMock.mock.calls[0][1] as (e: { payload: unknown }) => void;
    handler({ payload: { cpu_percent: 42 } });
    expect(cb).toHaveBeenCalledWith({ cpu_percent: 42 });
    unlisten();
    expect(off).toHaveBeenCalled();
  });
});
