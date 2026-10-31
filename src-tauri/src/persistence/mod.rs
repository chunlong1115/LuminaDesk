//! 持久化：JSON 原子写入（tmp→sync_all→rename，保留 .bak 回退）。
//! 阶段 1 交付本模块 + 单元测试；Store trait 在引入新存储后端时替换实现。

pub mod models;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialize error: {0}")]
    Serialize(#[from] serde_json::Error),
}

/// 原子写入：写 *.tmp → sync_all → rename 覆盖（Windows 上即原子替换）；
/// 覆盖前把现有文件复制为 *.bak。
pub fn atomic_write<P: AsRef<Path>, T: Serialize>(path: P, value: &T) -> Result<(), StoreError> {
    let path = path.as_ref();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = tmp_path(path);
    let json = serde_json::to_vec_pretty(value)?;

    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp)?;
    f.write_all(&json)?;
    f.sync_all()?;
    drop(f);

    // 覆盖前保留 .bak（存在旧文件才备份）
    if path.exists() {
        let bak = bak_path(path);
        let _ = fs::remove_file(&bak);
        fs::copy(path, &bak)?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

/// 读取：主文件损坏时自动回退 .bak；两者皆失败返回 None（调用方用默认值）。
pub fn load_with_fallback<P: AsRef<Path>, T: DeserializeOwned>(path: P) -> Option<T> {
    let path = path.as_ref();
    match fs::read(path).and_then(|bytes| serde_json::from_slice(&bytes).map_err(std::io::Error::other)) {
        Ok(v) => Some(v),
        Err(_) => {
            let bak = bak_path(path);
            fs::read(&bak)
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        }
    }
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".tmp");
    PathBuf::from(s)
}

fn bak_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".bak");
    PathBuf::from(s)
}

/// 供测试与清理使用
pub fn test_dir() -> PathBuf {
    std::env::temp_dir().join("luminadesk-persistence-tests")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static SEQ: AtomicU32 = AtomicU32::new(0);

    fn unique_file(name: &str) -> PathBuf {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        test_dir().join(format!("{name}-{n}.json"))
    }

    #[test]
    fn roundtrip_and_bak() {
        let p = unique_file("rt");
        atomic_write(&p, &serde_json::json!({"a": 1})).unwrap();
        let v: serde_json::Value = load_with_fallback(&p).unwrap();
        assert_eq!(v["a"], 1);

        // 第二次写：.bak 应保留旧内容
        atomic_write(&p, &serde_json::json!({"a": 2})).unwrap();
        let v2: serde_json::Value = load_with_fallback(&p).unwrap();
        assert_eq!(v2["a"], 2);
        let bak: serde_json::Value =
            serde_json::from_slice(&fs::read(bak_path(&p)).unwrap()).unwrap();
        assert_eq!(bak["a"], 1);
        fs::remove_dir_all(test_dir()).ok();
    }

    #[test]
    fn corruption_falls_back_to_bak() {
        let p = unique_file("corrupt");
        atomic_write(&p, &serde_json::json!({"ok": true})).unwrap();
        atomic_write(&p, &serde_json::json!({"ok": "second"})).unwrap();
        // 主文件写入垃圾字节
        std::fs::File::create(&p).unwrap().write_all(b"{{{garbage").unwrap();
        let v: serde_json::Value = load_with_fallback(&p).unwrap();
        // 回退到 .bak（第一次写入的内容）
        assert_eq!(v["ok"], true);
        fs::remove_dir_all(test_dir()).ok();
    }

    #[test]
    fn both_missing_returns_none() {
        let p = unique_file("missing");
        let v: Option<serde_json::Value> = load_with_fallback(&p);
        assert!(v.is_none());
    }
}
