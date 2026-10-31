//! 挂载看门狗（阶段 2 从 spike watchdog/reattach 移植）
//! TODO(阶段2): 5s 句柄校验 + 2s 冷却 + 60s/5 次限流 + 事件驱动重挂 +
//!              死窗口重建（唯一 label container-{monitor}-{gen}，spike 教训 4）
