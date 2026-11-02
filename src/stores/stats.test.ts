import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { StatsSample } from "../lib/ipc";
import { _resetForTest, toSample, useStats } from "./stats";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const invokeMock = invoke as unknown as ReturnType<typeof vi.fn>;
const listenMock = listen as unknown as ReturnType<typeof vi.fn>;

function lastHandler() {
  return listenMock.mock.calls.at(-1)![1] as (e: { payload: StatsSample }) => void;
}

describe("stats 环形缓冲", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    _resetForTest();
    useStats.setState({ history: [] });
    invokeMock.mockResolvedValue(1);
    listenMock.mockResolvedValue(vi.fn());
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

describe("toSample 载荷映射", () => {
  it("CPU 钳位 0..=100、字节→MB", () => {
    expect(
      toSample({
        cpu_percent: 123.4,
        mem_used_bytes: 512 * 1024 * 1024,
        mem_total_bytes: 16384 * 1024 * 1024,
      }),
    ).toEqual({ cpu: 100, memUsedMb: 512, memTotalMb: 16384 });
    expect(toSample({ cpu_percent: -5, mem_used_bytes: 0, mem_total_bytes: 0 })).toEqual({
      cpu: 0,
      memUsedMb: 0,
      memTotalMb: 0,
    });
  });
});

describe("stats 订阅接线", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    _resetForTest();
    useStats.setState({ history: [] });
    invokeMock.mockResolvedValue(1);
    listenMock.mockResolvedValue(vi.fn());
  });

  it("首个订阅启动采样线程；重复订阅不重复 invoke", async () => {
    useStats.getState().subscribe();
    await vi.waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("stats_subscribe", { enabled: true }),
    );
    await vi.waitFor(() => expect(listenMock).toHaveBeenCalled());
    invokeMock.mockClear();
    useStats.getState().subscribe();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("事件样本映射后入环形缓冲", async () => {
    useStats.getState().subscribe();
    await vi.waitFor(() => expect(listenMock).toHaveBeenCalled());
    const handler = lastHandler();
    handler({
      payload: {
        cpu_percent: 123.4,
        mem_used_bytes: 512 * 1024 * 1024,
        mem_total_bytes: 16384 * 1024 * 1024,
      },
    });
    expect(useStats.getState().history.at(-1)).toEqual({
      cpu: 100,
      memUsedMb: 512,
      memTotalMb: 16384,
    });
    handler({ payload: { cpu_percent: -3, mem_used_bytes: 1, mem_total_bytes: 2 } });
    expect(useStats.getState().history.at(-1)?.cpu).toBe(0);
  });

  it("归零退订：停摆线程并摘除监听；重复退订幂等", async () => {
    const off = vi.fn();
    listenMock.mockResolvedValue(off);
    useStats.getState().subscribe();
    await vi.waitFor(() => expect(listenMock).toHaveBeenCalled());
    useStats.getState().unsubscribe();
    expect(invokeMock).toHaveBeenCalledWith("stats_subscribe", { enabled: false });
    expect(off).toHaveBeenCalled();
    invokeMock.mockClear();
    off.mockClear();
    useStats.getState().unsubscribe();
    expect(invokeMock).not.toHaveBeenCalled();
    expect(off).not.toHaveBeenCalled();
  });

  it("订阅在途退订（StrictMode 双挂载）：true 落地后自动补退订", async () => {
    let resolveTrue!: (v: number) => void;
    invokeMock.mockImplementation(
      () => new Promise<number>((resolve) => (resolveTrue = resolve)),
    );
    useStats.getState().subscribe();
    useStats.getState().unsubscribe();
    expect(listenMock).not.toHaveBeenCalled();
    resolveTrue(1);
    await vi.waitFor(() =>
      expect(invokeMock).toHaveBeenLastCalledWith("stats_subscribe", { enabled: false }),
    );
    expect(listenMock).not.toHaveBeenCalled();
  });

  it("监听建立完成前全部退订：摘除监听并停摆", async () => {
    let resolveListen!: (off: () => void) => void;
    const off = vi.fn();
    listenMock.mockImplementation(
      () => new Promise<() => void>((resolve) => (resolveListen = resolve)),
    );
    useStats.getState().subscribe();
    await vi.waitFor(() => expect(listenMock).toHaveBeenCalled());
    useStats.getState().unsubscribe();
    resolveListen(off);
    await vi.waitFor(() =>
      expect(invokeMock).toHaveBeenLastCalledWith("stats_subscribe", { enabled: false }),
    );
    expect(off).toHaveBeenCalled();
  });
});
