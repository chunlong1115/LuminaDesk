# 阶段 3 浮层穿透模式（2026-11-02）

## 实现要点

- **hit_test 纯逻辑（C9a）**：`HitRegion` 共享 `RwLock<Vec<_>>`（WM_NCHITTEST 高频读、前端布局低频写）；`contains` 半开区间（含左/上、不含右/下）；`unpack_lparam` 按 16 位有符号分量拆包（负坐标副屏必须符号扩展）。
- **浮层窗口（C9b）**：`widgets-0` 独立 WebviewWindow——transparent + 无边框 + `WS_EX_NOACTIVATE`（点击不抢焦点）+ `WS_EX_TOOLWINDOW`（不进 alt-tab）；覆盖主显示器物理矩形、`HWND_BOTTOM` 置底（桌面图标之上、普通窗口之下）。`GWLP_WNDPROC` 子类化：`WM_NCHITTEST` 非命中区返回 `HTTRANSPARENT` 透传下层（桌面图标可点），命中区正常接收；`WM_KILLFOCUS` 自动恢复免激活。原窗口过程按 hwnd 存 `OnceLock<Mutex<HashMap>>` 惰性表。
- **windows 0.61 API 差异（坑）**：`ScreenToClient` 在 `Graphics::Gdi` 模块且返回 BOOL（无 `is_err`）；`SetWindowPos` 的 `hwndInsertAfter` 是 `Option<HWND>`；`WNDPROC = Option<unsafe extern "system" fn>`；`HashMap::new` 非 const → static 需 `OnceLock`；函数指针转整数 lint → `wnd_proc as *const () as isize`。
- **命令接线（C9c）**：`hit_test_set_regions`（整体替换命中区）/ `widgets_set_active`（摘挂 NOACTIVATE + `SetForegroundWindow`）/ `system_set_widgets_above_icons`（持久化 `widgets_in_container` 反转 + 创建/销毁浮层窗口）；启动 setup 按持久化设置重建浮层。
- **前端上报（C9d/C9e）**：ipc 封装三命令 + `HitRegion` 类型；`hitRegions` 纯函数 CSS px × dpr → 物理整数区（左/上 floor、右/下 ceil——宁外扩 1px 不漏点击）+ `sameRegions` 去重；`WidgetsPage` 渲染 `WidgetsLayer`，`MutationObserver`（style/class/childList）+ resize + 50ms 去抖 → 上报，Rust 侧零 DPI 换算。
- **命中区标注与输入激活（C9f）**：`data-hit-region` 挂卡片/便签/新便签钮；便签文本域与待办输入聚焦 → 仅浮层窗口（`?role=widgets` 守卫）摘 NOACTIVATE 抢键盘焦点、失焦恢复（NOACTIVATE 窗口拿不到键盘输入，必须摘除才能打字）。

## 验证方式

- cargo test：**25/25 通过**（hit_test 4 例新增）
- vitest：**62/62 通过**（ipc 3 例、hitRegions 6 例、windowRole 4 例新增）
- `pnpm typecheck`、`pnpm lint`：通过；cargo check 零新警告（Win32 密集模块与 host/attach 同约定不做单测）
- **E2E 人工验证（待用户执行）**：控制台切「浮于图标之上」→ 组件区可点、空白区点击落到桌面图标（框选/打开图标）；便签/待办可键盘输入；切回容器模式窗口销毁、组件回到容器

## Commit 清单

C9a hit_test 纯逻辑 → C9b 浮层窗口落地 → C9c 命令接线 → C9d ipc 封装 → C9e 命中区上报 → C9f 标注+输入激活 → C9g 文档

## 遗留（后续任务）

- 多显示器拓扑（v1 仅主显示器）
- 控制台模式切换开关 UI：任务 4（组件管理页 + widgets.json + 便签/待办 Rust CRUD）
