import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { DEFAULT_WIDGETS, _resetForTest, useWidgets } from "./widgets";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const invokeMock = invoke as unknown as ReturnType<typeof vi.fn>;
const listenMock = listen as unknown as ReturnType<typeof vi.fn>;

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

describe("useWidgets Rust 持久化接线", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // hydrate 同时拉配置与模式：按命令名分别返回
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "widgets_list") return Promise.resolve([]);
      if (cmd === "widgets_mode_get") return Promise.resolve(false);
      return Promise.resolve(undefined);
    });
    listenMock.mockResolvedValue(() => {});
    useWidgets.setState({ widgets: DEFAULT_WIDGETS, aboveIcons: false });
    _resetForTest();
  });

  it("hydrate 加载 Rust 配置与模式并订阅两个广播", async () => {
    const rustList = [{ id: "clock", kind: "clock" as const, x: 10, y: 20 }];
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "widgets_list") return Promise.resolve(rustList);
      if (cmd === "widgets_mode_get") return Promise.resolve(true);
      return Promise.resolve(undefined);
    });
    useWidgets.getState().hydrate();
    await vi.waitFor(() => {
      expect(useWidgets.getState().widgets).toEqual(rustList);
    });
    expect(useWidgets.getState().aboveIcons).toBe(true);
    expect(invokeMock).toHaveBeenCalledWith("widgets_list");
    expect(invokeMock).toHaveBeenCalledWith("widgets_mode_get");
    expect(listenMock).toHaveBeenCalledWith("lumina://widgets-changed", expect.any(Function));
    expect(listenMock).toHaveBeenCalledWith(
      "lumina://widgets-mode-changed",
      expect.any(Function),
    );
  });

  it("widgets-changed 广播到达后镜像更新", async () => {
    useWidgets.getState().hydrate();
    await vi.waitFor(() => {
      expect(listenMock).toHaveBeenCalled();
    });
    const handler = listenMock.mock.calls[0]![1] as (e: { payload: unknown }) => void;
    const broadcast = [{ id: "todo", kind: "todo" as const, x: 999, y: 888 }];
    handler({ payload: broadcast });
    expect(useWidgets.getState().widgets).toEqual(broadcast);
  });

  it("widgets-mode-changed 广播更新 aboveIcons（容器窗口据此隐藏/显示组件层）", async () => {
    useWidgets.getState().hydrate();
    await vi.waitFor(() => {
      expect(listenMock.mock.calls.length).toBeGreaterThanOrEqual(2);
    });
    const handler = listenMock.mock.calls[1]![1] as (e: { payload: unknown }) => void;
    handler({ payload: true });
    expect(useWidgets.getState().aboveIcons).toBe(true);
    handler({ payload: false });
    expect(useWidgets.getState().aboveIcons).toBe(false);
  });

  it("hydrate 幂等：重复调用配置只 invoke 一次", async () => {
    useWidgets.getState().hydrate();
    useWidgets.getState().hydrate();
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalled();
    });
    expect(invokeMock.mock.calls.filter((c) => c[0] === "widgets_list")).toHaveLength(1);
  });
});
