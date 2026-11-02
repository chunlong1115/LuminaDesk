# 阶段 3 五组件前端（2026-11-02）

## 实现要点

- **stats store 订阅接线**：订阅引用计数 0→1 invoke `stats_subscribe(true)` 并监听 `lumina://stats-sample`，归零停摆摘除监听（幂等）。双挂载竞态防护：订阅在途/监听未建时退订 → 落地后自动补退订。载荷映射 `toSample`（CPU 钳位 0..=100、字节→MB）入 60 点环形缓冲。
- **时钟**：首帧 `msToNextSecond` 对齐秒沿（防相位漂移），此后 1s 间隔——仅整秒重渲染；**日期**：60s 低频自检，仅跨日重渲染。
- **CPU/内存**：`sparkPoints` 纯函数坐标映射（0..=max 归一、全零不除零）+ 双 SVG 迷你图读环形缓冲。
- **便签**：多张可拖拽缩放；store 纯函数 `clampNoteSize` 钳位下限；拖拽沿用容器经验（pointer capture + ref 记起点，勿读异步 state）；文本域/删除键隔离拖拽。
- **待办**：store 增删勾选 + 空文本校验纯函数；输入框回车/按钮添加。
- **组件层**：`WidgetsLayer` 合并进容器窗口（图标之上 z-index 5）；层 pointer-events:none 穿透空白区（图标框选/拖拽不受影响）；默认布局 `DEFAULT_WIDGETS` 右侧列。
- **格式化**：`lib/format.ts`（clampPct/formatMb/formatClock/msToNextSecond/formatDate）组件展示逻辑全下沉纯函数。

## 验证方式

- vitest：**49/49 通过**（10 文件；@tauri-apps/api mock：订阅接线/竞态 6 例）
- `pnpm typecheck`、`pnpm lint`：通过
- cargo test：**21/21 通过**（含 stats 差值边界 6 例）

## Commit 清单

C8a ipc 封装 → C8b lib/format → C8c stats store 接线 → C8d 时钟/日期 → C8e CPU/内存+迷你图 → C8f 便签 → C8g 待办 → C8h 组件层合并容器 → C8i/C8j 用例修正 → C8k stats refCountNow（TS 流收窄）→ C8l/C8m/C8n 用例非空断言 → C8o 回绕用例常数修正

## 遗留（后续任务）

- 便签/待办内容持久化：待 Rust 便签待办 CRUD 命令（阶段 3 任务 4 前）
- 组件管理页（增删/位置/模式切换）+ widgets.json 持久化：任务 4
- 「浮于图标之上」模式：widgets 窗口 + WM_NCHITTEST 穿透：任务 3（下一步）
