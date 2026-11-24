# 阶段 3 组件管理器（2026-11-03）

## 实现要点

- **Rust 持久化层（C10a）**：`commands/widgets.rs`——widgets.json 单文件三区（widgets/notes/todos）；复用 persistence 原子写 + 损坏回退 .bak；组件配置缺失文件回退默认五组件（与前端 DEFAULT_WIDGETS 对齐），便签/待办回退空；`is_valid_kind` 校验五类组件。九命令：`widgets_list/upsert/remove`、`notes_list/upsert/remove`、`todos_list/upsert/remove`。**广播只挂组件配置写路径**（`lumina://widgets-changed`，载荷为全量配置列表——管理页改 → 容器/浮层窗口镜像）；便签/待办单窗口编辑不广播。
- **前端 store 接线（C10b）**：ipc 九命令封装 + `WidgetConfig/NoteItem/TodoItem/WidgetKind` 类型（与 Rust 对齐）；widgets store `hydrate()` 幂等水合（加载 + 订阅广播镜像）；**便签 300ms 去抖 upsert**（打字/拖拽高频），add/remove 直写；**串行写链**保证删除晚于在途 upsert 不复活，删除即取消待去抖项；待办增删勾低频直写；水合失败复位允许重试；挂载点：WidgetsLayer/Notes/Todo 各一行 useEffect（StrictMode 双挂载由模块级幂等标志防护）。
- **管理面板（C10c）**：control「组件管理」卡片——类型下拉 + 添加（`nextWidgetPos` 阶梯偏移防重叠、`newWidgetId` 时间戳防冲突）；每行 X/Y 输入失焦/回车提交 upsert，`parsePos` 非法输入回退原值、无变化不 invoke；「浮于图标之上」开关走 `systemSetWidgetsAboveIcons` + 同步 settings store 镜像；Rust 补 `widgets_mode_get` 读取（现有 IPC 无读侧）。
- **测试修复（C10b-fix）**：临时目录测试用确定性目录名未清理 → 二次运行读残留失败；`unique_dir` 预清理 + 用例末 remove_dir_all，连续两轮全绿验证。

## 验证方式

- cargo test：**34/34**（widgets 持久化 9 例新增；连续两轮运行稳定）
- vitest：**83/83**（ipc 映射 3 + widgets 水合/广播/幂等 3 + 便签 4 + 待办 4 + widgetPanel 5 + widgetsModeGet 1）
- `pnpm typecheck`、`pnpm lint`：通过；cargo check 零错误零新警告
- **E2E 人工验证（待用户执行）**：管理页增删/调位 → 容器（或浮层）窗口即时反映（广播）；重启后 widgets.json 回读；模式切换创建/销毁浮层窗口；便签/待办重启后内容保留

## Commit 清单

C10a Rust 持久化层 → C10b store 数据链路 → C10b-fix 测试目录残留 → C10c 管理面板 → C10d 文档

## 遗留（后续任务）

- ✅ 模式切换的容器窗口联动：已由 C10e 修复（`system_set_widgets_above_icons` 成功后广播 `widgets-mode-changed`；widgets store 水合读模式并订阅广播；ContainerPage 按 aboveIcons 隐藏/显示 WidgetsLayer，水合由容器窗口触发，浮层模式不重复渲染）
- 可选 `system_hide_desktop_icons`（注册表 + 通知 explorer，失败降级提示）
- C6 24H2 虚拟机兼容性验证：阶段 5 前补做
