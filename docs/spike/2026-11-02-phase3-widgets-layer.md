# 阶段 3 浮层穿透模式（2026-11-02）

## 实现要点

- **hit_test 纯逻辑（C9a）**：`HitRegion` 共享 `RwLock<Vec<_>>`（WM_NCHITTEST 高频读、前端布局低频写）；`contains` 半开区间（含左/上、不含右/下）；`unpack_lparam` 按 16 位有符号分量拆包（负坐标副屏必须符号扩展）。
- **浮层窗口（C9b）**：`widgets-0` 独立 WebviewWindow——transparent + 无边框 + `WS_EX_NOACTIVATE`（点击不抢焦点）+ `WS_EX_TOOLWINDOW`（不进 alt-tab）；覆盖主显示器物理矩形、`HWND_BOTTOM` 置底（桌面图标之上、普通窗口之下）。`GWLP_WNDPROC` 子类化：`WM_NCHITTEST` 非命中区返回 `HTTRANSPARENT` 透传下层（桌面图标可点），命中区正常接收；`WM_KILLFOCUS` 自动恢复免激活。原窗口过程按 hwnd 存 `OnceLock<Mutex<HashMap>>` 惰性表。
- **windows 0.61 API 差异（坑）**：`ScreenToClient` 在 `Graphics::Gdi` 模块且返回 BOOL（无 `is_err`）；`SetWindowPos` 的 `hwndInsertAfter` 是 `Option<HWND>`；`WNDPROC = Option<unsafe extern "system" fn>`；`HashMap::new` 非 const → static 需 `OnceLock`；函数指针转整数 lint → `wnd_proc as *const () as isize`。
- **命令接线（C9c）**：`hit_test_set_regions`（整体替换命中区）/ `widgets_set_active`（摘挂 NOACTIVATE + `SetForegroundWindow`）/ `system_set_widgets_above_icons`（持久化 `widgets_in_container` 反转 + 创建/销毁浮层窗口）；启动 setup 按持久化设置重建浮层。
- **前端上报（C9d/C9e）**：ipc 封装三命令 + `HitRegion` 类型；`hitRegions` 纯函数 CSS px × dpr → 物理整数区（左/上 floor、右/下 ceil——宁外扩 1px 不漏点击）+ `sameRegions` 去重；`WidgetsPage` 渲染 `WidgetsLayer`，`MutationObserver`（style/class/childList）+ resize + 50ms 去抖 → 上报，Rust 侧零 DPI 换算。
- **命中区标注与输入激活（C9f）**：`data-hit-region` 挂卡片/便签/新便签钮；便签文本域与待办输入聚焦 → 仅浮层窗口（`?role=widgets` 守卫）摘 NOACTIVATE 抢键盘焦点、失焦恢复（NOACTIVATE 窗口拿不到键盘输入，必须摘除才能打字）。

### E2E 双 bug 修复（C11b / C11c，用户 E2E 反馈「桌面不能进行任何操作」「没有适应桌面大小」）

- **几何精确满屏（C11b）——「没有适应桌面大小」根因**：两个独立病灶，逐一实测定位——
  1. **WebView2 dpr 冻结 1.0**：旧实现 visible(false)+默认尺寸建窗再原生 SetWindowPos 改几何，WebView2 缩放因子按初始上下文冻结（125% 下组件不随桌面缩放）。修：建窗参数直接按主显示器**逻辑几何**落地（`monitor_from_point(0,0)` + `physical_to_logical` 折算 position/inner_size）。
  2. **client 偏移 (8,1) 且右/下缘裁边**：tao 0.37.2 对 undecorated 顶层窗口默认 `decoration_shadow=true`（PlatformSpecific 默认值，tauri 未暴露关闭入口），其 WM_NCCALCSIZE 按 `calculate_insets_for_dpi` 把 client 内缩 (8,1,8,8)（96dpi 实测；`outer=1936×1089 / client=1920×1080`、WRY_WEBVIEW 子窗贴 (8,1)）。修：子类化**先于**几何重排挂载、WM_NCCALCSIZE 直接返回 0（整窗即客户区，不转发 tao）；剥标题栏/调整边框类样式（免 Snap Layouts 悬浮/系统菜单，纯函数 `borderless_style`）；最后显式 `SetWindowPos` 到物理屏幕矩形 + SWP_FRAMECHANGED。实测 outer=client=子窗=(0,0)-(1920,1080) 逐像素吻合。
- **z 序桌面可操作（C11c）——「桌面不能进行任何操作」根因**：旧 set_active 摘 NOACTIVATE + SetForegroundWindow 抬顶后**等 blur 才复位**——窗口霸顶期间透明区点击透传被其上方的全屏窗口（用户 Chrome）吞掉，且 blur 永不送达 → 桌面点死。修：SetForegroundWindow 抢键盘焦点后**立即** HWND_BOTTOM 置底收回桌面带（z 序与激活态正交——物理键盘只路由到前台线程的焦点窗口，置底不改变激活态，输入期间桌面依然可点）；NOACTIVATE 全程保留；WM_KILLFOCUS 恢复免激活 + 防御性置底、WM_WINDOWPOSCHANGING 守卫强制一切抬窗置底（双保险，纯函数 `force_bottom_on_change`）。

## 验证方式

- cargo test：**37/37 通过**（新增 physical_to_logical 3 例、borderless_style 3 例、force_bottom_on_change 5 例）
- vitest：**84/84 通过**；`pnpm typecheck`、`pnpm lint`：通过；cargo check 零错误（存量 23 警告与本次无关）
- **Win32 探测回归（100% 环境，全部通过）**：
  - `client_rect.ps1`：outer=client=(0,0)-(1920,1080)，WRY_WEBVIEW 及 WebView2 子窗全部 (0,0)-(1920,1080)（修复前子窗 (8,1)-(1928,1081)）
  - `grid_scan.ps1`：38 个 HTCLIENT 命中点全部落在组件卡片区内，其余网格点全部透传
  - `hit_test_bounds.ps1`：待办卡片渲染矩形（content-box 290×342）边界逐像素判定 11/11 PASS（含左/上含、右/下不含的半开区间语义）
  - `fg_mechanics_test.ps1`：SetForegroundWindow 成功（fg=浮层）→ 窗口仍处桌面带（rank≈373/624、上方即桌面窗口）→ KILLFOCUS 复位置底 + NOACTIVATE 保持 → 前台归还（Chrome 0x10880）
  - `zwatch.ps1`：持续观察窗口稳定于桌面带，无抬窗
- **待用户桌面态复测**：125% 缩放下 dpr 修复效果（当前环境 100%，需切缩放）；DOM 点击→打字→持久化链路（全屏 Chrome 遮挡鼠标注入，机制已探针验证）

## Commit 清单

C9a hit_test 纯逻辑 → C9b 浮层窗口落地 → C9c 命令接线 → C9d ipc 封装 → C9e 命中区上报 → C9f 标注+输入激活 → C9g 文档 → C11b 几何精确满屏修复 → C11c z 序桌面可操作修复

## 遗留（后续任务）

- 多显示器拓扑（v1 仅主显示器）
- 控制台模式切换开关 UI：✅ 任务 4 完成（C10c 管理面板，见 2026-11-03-phase3-widget-manager.md）
- 模式切换的容器窗口联动（浮层模式双窗口重复渲染）：✅ C10e 完成（widgets-mode-changed 广播 + 容器按 aboveIcons 隐藏组件层）
- 125% 缩放修复效果需用户切缩放复测（当前机器 100%）
- 容器窗口同样的 tao shadow client 内缩 (8,1,8,8) 偏移未修（C10 容器渲染范畴，浮层已修）
- 容器窗口 125% 下可能的 dpr 冻结/attach 物理尺寸问题未查（与 C11b 病灶 1 同类，待查）
