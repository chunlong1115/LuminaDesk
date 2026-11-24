# LuminaDesk（内部代号 LMDesk）

Windows 11 x64 桌面美化与效率软件：桌面容器（壁纸层、图标之下）、小组件、文件整理（**默认不移动用户原始文件**）、低资源占用。

当前处于 **阶段 3**：五组件（时钟/日期/CPU·内存/便签/待办）、浮层穿透模式、组件管理器已完成开发，整体 E2E 验收进行中。

## 技术栈

- Rust + Tauri 2（desktop layer 自研：WorkerW 挂载 + 看门狗 + 事件驱动重挂）
- React 18 + TypeScript（strict）+ Vite + zustand + CSS Modules
- 持久化：JSON 原子写入（.tmp→rename，.bak 回退）；无 SQLite、无 tokio、无组件库

## 开发

```bash
pnpm install
pnpm tauri dev      # 开发（控制窗口 + 托盘）
pnpm test           # 前端 vitest
pnpm typecheck      # TypeScript 类型检查
pnpm lint           # ESLint
cargo test          # Rust 单测（src-tauri 下）
```

## 目录结构

| 路径 | 说明 |
|---|---|
| `src/` | React 前端：容器/浮层/控制台三个窗口视图，组件与 zustand store |
| `src-tauri/src/desktop_layer/` | WorkerW 挂载、几何、看门狗、重挂 |
| `src-tauri/src/widgets_layer.rs` | 浮层窗口与 `WM_NCHITTEST` 穿透 |
| `src-tauri/src/commands/` | 容器与组件配置 IPC 命令 |
| `src-tauri/src/persistence/` | JSON 原子写与损坏回退 |
| `docs/spike/` | 阶段 0–3 的实测报告、方案与验收清单（项目权威上下文） |
| `scripts/` | 阶段 2 回归脚本（点击免激活 / Win+D / 防节流遮挡） |
| `.e2e/` | 阶段 3 E2E 交互探针（窗口枚举、输入注入、桌面图标命中判定） |
| `.diag/` | Win32 诊断探针（几何 / DPI / z 序 / 焦点 / 命中区边界） |

## 回归与诊断脚本

PowerShell 5.1 脚本，全部 **ASCII-only**（PS 5.1 会按 GBK 解析无 BOM UTF-8，中文注释可能吞换行）；坐标均为物理像素，脚本内部自行提升 DPI awareness。

```powershell
# 阶段 2 回归
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\s3-noactivate.ps1

# 阶段 3 交互探针（每个 pwsh 进程需重新 dot-source）
. .\.e2e\e2e.ps1
. .\.e2e\lumina.ps1

# Win32 诊断
powershell -NoProfile -ExecutionPolicy Bypass -File .diag\client_rect.ps1
```

详细场景与预期结果见 [`docs/spike/2026-11-03-phase3-e2e-checklist.md`](docs/spike/2026-11-03-phase3-e2e-checklist.md)。

## 文档

- 阶段 0 Spike 实测报告（S1–S13、四大根因修复）：[`docs/spike/2026-10-31.md`](docs/spike/2026-10-31.md)
- 阶段 2 收尾回归：[`docs/spike/2026-11-01.md`](docs/spike/2026-11-01.md)
- 阶段 3 五组件：[`docs/spike/2026-11-02-phase3-widgets.md`](docs/spike/2026-11-02-phase3-widgets.md)
- 阶段 3 浮层穿透：[`docs/spike/2026-11-02-phase3-widgets-layer.md`](docs/spike/2026-11-02-phase3-widgets-layer.md)
- 阶段 3 组件管理器：[`docs/spike/2026-11-03-phase3-widget-manager.md`](docs/spike/2026-11-03-phase3-widget-manager.md)
- 阶段 3 E2E 验收清单：[`docs/spike/2026-11-03-phase3-e2e-checklist.md`](docs/spike/2026-11-03-phase3-e2e-checklist.md)

## 许可证

[MIT](LICENSE)
