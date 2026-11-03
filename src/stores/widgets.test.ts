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
    invokeMock.mockResolvedValue([]);
    listenMock.mockResolvedValue(() => {});
    useWidgets.setState({ widgets: DEFAULT_WIDGETS });
    _resetForTest();
  });

  it("hydrate 加载 Rust 配置并订阅 widgets-changed 广播", async () => {
    const rustList = [{ id: "clock", kind: "clock" as const, x: 10, y: 20 }];
    invokeMock.mockResolvedValue(rustList);
    useWidgets.getState().hydrate();
    await vi.waitFor(() => {
      expect(useWidgets.getState().widgets).toEqual(rustList);
    });
    expect(invokeMock).toHaveBeenCalledWith("widgets_list");
    expect(listenMock).toHaveBeenCalledWith("lumina://widgets-changed", expect.any(Function));
  });

  it("广播到达后镜像更新", async () => {
    invokeMock.mockResolvedValue([]);
    useWidgets.getState().hydrate();
    await vi.waitFor(() => {
      expect(listenMock).toHaveBeenCalled();
    });
    const handler = listenMock.mock.calls[0]![1] as (e: { payload: unknown }) => void;
    const broadcast = [{ id: "todo", kind: "todo" as const, x: 999, y: 888 }];
    handler({ payload: broadcast });
    expect(useWidgets.getState().widgets).toEqual(broadcast);
  });

  it("hydrate 幂等：重复调用只 invoke 一次", async () => {
    useWidgets.getState().hydrate();
    useWidgets.getState().hydrate();
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledTimes(1);
    });
  });
});
