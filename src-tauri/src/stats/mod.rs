//! CPU/内存采样器：前端订阅引用计数 >0 时启动 1Hz 采样线程，归零即停（无订阅停摆）。
//! CPU 占用率 = GetSystemTimes 两次采样差值（sysinfo 0.33 不暴露原始计数器，无法做差）；
//! 内存 = sysinfo。采样结果经 lumina://stats-sample 事件推送（前端零轮询）。

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::events::STATS_SAMPLE;

/// 采样间隔：严格 1Hz
pub const SAMPLE_INTERVAL_MS: u64 = 1000;

/// GetSystemTimes 原始计数器快照（累计 100ns 刻度）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuTimes {
    /// busy = kernel + user − idle（累计）
    pub busy: u64,
    /// idle（累计）
    pub idle: u64,
}

/// 两次采样差值 → 全局 CPU 占用率（0..=100）。
/// 计数器回绕（u64 溢出）由 wrapping 语义容忍；差值总量为 0 时返回 0 而非 NaN。
pub fn cpu_percent_delta(prev: CpuTimes, cur: CpuTimes) -> f32 {
    let d_busy = cur.busy.wrapping_sub(prev.busy);
    let d_idle = cur.idle.wrapping_sub(prev.idle);
    let d_total = d_busy.saturating_add(d_idle);
    if d_total == 0 {
        0.0
    } else {
        ((100.0 * d_busy as f64) / (d_total as f64)).clamp(0.0, 100.0) as f32
    }
}

/// 单次采样推送载荷（lumina://stats-sample）
#[derive(Debug, Clone, Copy, Serialize)]
pub struct StatsSample {
    /// 全局 CPU 占用率（0..=100）
    pub cpu_percent: f32,
    /// 已用物理内存（字节）
    pub mem_used_bytes: u64,
    /// 物理内存总量（字节）
    pub mem_total_bytes: u64,
}

/// 订阅引用计数：>0 时采样线程存活
static SUBSCRIBERS: AtomicUsize = AtomicUsize::new(0);
/// 停止标志：计数归零时置位，采样线程醒来即退
static STOP: AtomicBool = AtomicBool::new(false);
/// 订阅/退订的启停动作互斥（多窗口并发调用时串行化）
static LIFECYCLE: Mutex<()> = Mutex::new(());
/// 采样线程句柄（stop 时 join 用）
static HANDLE: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);

/// 前端订阅（enabled=true）或退订（false）；返回当前引用计数。
/// 计数 0→1 启动采样线程；1→0 置停止位并 join（阻塞 ≤1 个采样间隔）。退订幂等。
pub fn set_subscribed(app: AppHandle, enabled: bool) -> usize {
    let _guard = LIFECYCLE.lock().unwrap();
    if enabled {
        let n = SUBSCRIBERS.fetch_add(1, Ordering::SeqCst) + 1;
        if n == 1 && start_sampler(app).is_err() {
            // 启动失败：回滚计数，下次订阅重试
            SUBSCRIBERS.fetch_sub(1, Ordering::SeqCst);
            return 0;
        }
        n
    } else {
        let prev = SUBSCRIBERS
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |c| {
                if c > 0 {
                    Some(c - 1)
                } else {
                    None
                }
            })
            .unwrap_or(0);
        let n = prev.saturating_sub(1);
        if prev > 0 && n == 0 {
            stop_sampler();
        }
        n
    }
}

fn start_sampler(app: AppHandle) -> std::io::Result<()> {
    STOP.store(false, Ordering::Relaxed);
    let handle = std::thread::Builder::new()
        .name("lumina-stats".into())
        .spawn(move || sample_loop(app))?;
    *HANDLE.lock().unwrap() = Some(handle);
    Ok(())
}

fn stop_sampler() {
    STOP.store(true, Ordering::Relaxed);
    if let Some(handle) = HANDLE.lock().unwrap().take() {
        let _ = handle.join(); // 最多等待一个采样间隔
    }
}

fn raw_cpu_times() -> Option<CpuTimes> {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::GetSystemTimes;

    let (mut idle, mut kernel, mut user) =
        (FILETIME::default(), FILETIME::default(), FILETIME::default());
    unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)).ok()? };
    let ticks = |ft: FILETIME| (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime);
    let idle = ticks(idle);
    let busy = ticks(kernel).wrapping_add(ticks(user)).wrapping_sub(idle);
    Some(CpuTimes { busy, idle })
}

/// 采样主循环：每秒一次差值采样并广播，STOP 置位即退
fn sample_loop(app: AppHandle) {
    let mut sys = sysinfo::System::new();
    let mut prev = raw_cpu_times();
    while !STOP.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(SAMPLE_INTERVAL_MS));
        if STOP.load(Ordering::Relaxed) {
            break;
        }
        sys.refresh_memory();
        let cur = raw_cpu_times();
        if cur.is_none() {
            log::warn!("GetSystemTimes failed, cpu_percent=0");
        }
        let cpu_percent = match (prev, cur) {
            (Some(p), Some(c)) => cpu_percent_delta(p, c),
            _ => 0.0,
        };
        prev = cur;
        let sample = StatsSample {
            cpu_percent,
            mem_used_bytes: sys.used_memory(),
            mem_total_bytes: sys.total_memory(),
        };
        if let Err(e) = app.emit(STATS_SAMPLE, sample) {
            log::warn!("stats-sample emit failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_is_strict_1hz() {
        assert_eq!(SAMPLE_INTERVAL_MS, 1000);
    }

    #[test]
    fn delta_half_busy() {
        let prev = CpuTimes {
            busy: 1000,
            idle: 1000,
        };
        let cur = CpuTimes {
            busy: 1500,
            idle: 1500,
        };
        assert!((cpu_percent_delta(prev, cur) - 50.0).abs() < 0.001);
    }

    #[test]
    fn delta_idle_only_is_zero() {
        let prev = CpuTimes {
            busy: 1000,
            idle: 1000,
        };
        let cur = CpuTimes {
            busy: 1000,
            idle: 2000,
        };
        assert_eq!(cpu_percent_delta(prev, cur), 0.0);
    }

    #[test]
    fn delta_full_busy_is_100() {
        let prev = CpuTimes {
            busy: 1000,
            idle: 1000,
        };
        let cur = CpuTimes {
            busy: 1100,
            idle: 1000,
        };
        assert_eq!(cpu_percent_delta(prev, cur), 100.0);
    }

    #[test]
    fn delta_zero_total_is_zero_not_nan() {
        let t = CpuTimes {
            busy: 1000,
            idle: 1000,
        };
        assert_eq!(cpu_percent_delta(t, t), 0.0);
    }

    #[test]
    fn delta_counter_wrap_stays_in_range() {
        // 计数器回绕（理论场景，u64 溢出）：wrapping 语义下结果仍落在 0..=100 且非 NaN
        let prev = CpuTimes {
            busy: u64::MAX - 50,
            idle: u64::MAX - 100,
        };
        let cur = CpuTimes { busy: 50, idle: 100 };
        let p = cpu_percent_delta(prev, cur);
        assert!(p.is_finite() && (0.0..=100.0).contains(&p));
        // d_busy=101, d_idle=200 → 101/301*100 ≈ 33.55
        assert!((p as f64 - 10100.0 / 301.0).abs() < 0.001);
    }
}
