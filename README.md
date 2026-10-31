# LuminaDesk（内部代号 LMDesk）

Windows 11 x64 桌面美化与效率软件：桌面容器（壁纸层、图标之下）、小组件、文件整理（默认不移动用户原始文件）、低资源占用。

## 技术栈

- Rust + Tauri 2（desktop layer 自研：WorkerW 挂载 + 看门狗 + 事件驱动重挂）
- React 18 + TypeScript（strict）+ Vite + zustand + CSS Modules
- 持久化：JSON 原子写入（.tmp→rename，.bak 回退）；无 SQLite、无 tokio、无组件库

## 开发

```bash
pnpm install
pnpm tauri dev      # 开发（控制窗口 + 托盘）
pnpm test           # 前端 vitest
cargo test          # Rust（src-tauri 下）
```

## 文档

- 架构与阶段计划：`docs/` + Claude 计划文件
- 阶段 0 Spike 实测报告（S1-S13、四大根因修复）：`docs/spike/2026-10-31.md`
