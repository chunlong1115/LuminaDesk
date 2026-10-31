# E2E 回归脚本

阶段 2 验收回归用 PowerShell 脚本（按 spike 方法论，实测有效）。

## 前置条件

- `pnpm tauri dev` 运行中（容器已挂载：dev 日志出现 `attach ok`）
- 脚本要求 **ASCII-only**（PS 5.1 按 GBK 解析无 BOM UTF-8，中文注释尾字节会吞换行）
- 屏幕 125% DPI：脚本内部已调用 `SetProcessDpiAwareness(2)`，全部坐标物理像素

## 脚本

| 脚本 | 场景 | 预期输出 |
|---|---|---|
| `s3-noactivate.ps1` | S3 点击免激活 | `PASS: S3 no-activation` |
| `s6-wind.ps1` | S6 Win+D ×2 往返 | `PASS: S6 Win+D roundtrip` |
| `occlude.ps1 [秒]` | S4 遮挡器 | 60s 黑色 TopMost 矩形；期间 dev 日志 `diag tick` 应每秒递增 |

## 用法

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\s3-noactivate.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\s6-wind.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\occlude.ps1 60
```

## 环境坑（本机实测）

- 本机 explorer 被 `taskkill /f` 后**不自动重启**（S5 回归需手动 `start explorer.exe`）
- s3 点击点由脚本在容器右下象限自动探测「壁纸空白点」（桌面图标散布需避开）
- 若 VS Code 等窗口最大化覆盖桌面：s3 脚本用 Win+D 暴露桌面后再点击，测完自动恢复
