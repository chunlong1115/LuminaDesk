//! 容器命令：桌面项只读列表、虚拟移动（不改文件）、布局持久化、打开/定位、虚拟分组。

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use base64::Engine;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
};
use windows::Win32::UI::Shell::{
    SHGetFileInfoW, SHGetKnownFolderPath, FOLDERID_Desktop, KF_FLAG_DEFAULT, SHGFI_ICON,
    SHGFI_SMALLICON, SHFILEINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::{DrawIconEx, DI_NORMAL};

use crate::persistence::{atomic_write, load_with_fallback};

/// 容器布局条目（物理像素；虚拟移动不改动文件本身）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutEntry {
    pub path: String,
    pub x: i32,
    pub y: i32,
}

/// 虚拟分组
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupEntry {
    pub id: String,
    pub name: String,
    pub item_paths: Vec<String>,
}

/// 容器持久化数据（containers.json）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContainerStore {
    pub layout: Vec<LayoutEntry>,
    pub groups: Vec<GroupEntry>,
}

#[derive(Debug, Serialize)]
pub struct ContainerItem {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
}

// ---- 内部工具 ----

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|d| d.join("containers.json"))
        .map_err(|e| e.to_string())
}

/// 纯函数：从指定路径读取 store（损坏回退 .bak，均失败返回默认值）——可单测
pub(crate) fn load_store_at(path: &std::path::Path) -> ContainerStore {
    load_with_fallback(path).unwrap_or_default()
}

/// 纯函数：原子写入指定路径——可单测
pub(crate) fn save_store_at(path: &std::path::Path, store: &ContainerStore) -> Result<(), String> {
    atomic_write(path, store).map_err(|e| e.to_string())
}

fn load_store(app: &AppHandle) -> ContainerStore {
    store_path(app)
        .ok()
        .map(|p| load_store_at(&p))
        .unwrap_or_default()
}

fn save_store(app: &AppHandle, store: &ContainerStore) -> Result<(), String> {
    let p = store_path(app)?;
    save_store_at(&p, store)
}

pub(crate) fn desktop_dir_path() -> Result<PathBuf, String> {
    desktop_dir()
}

fn desktop_dir() -> Result<PathBuf, String> {
    unsafe {
        let path = SHGetKnownFolderPath(&FOLDERID_Desktop, KF_FLAG_DEFAULT, None)
            .map_err(|e| format!("SHGetKnownFolderPath: {e}"))?;
        let s = path.to_string().map_err(|e| format!("path to_string: {e}"))?;
        Ok(PathBuf::from(s))
    }
}

/// 跳过 .ini（desktop.ini）与 .lnk 目标解析（v1 直接展示）
fn list_desktop() -> Result<Vec<ContainerItem>, String> {
    let dir = desktop_dir()?;
    let mut items = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "desktop.ini" {
            continue;
        }
        let md = entry.metadata().map_err(|e| e.to_string())?;
        items.push(ContainerItem {
            name,
            path: entry.path().to_string_lossy().into_owned(),
            is_dir: md.is_dir(),
            size: md.len(),
            modified: md
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
        });
    }
    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(items)
}

// ---- 命令 ----

#[tauri::command]
pub fn container_list_items() -> Result<Vec<ContainerItem>, String> {
    list_desktop()
}

/// 虚拟移动：只更新布局存储，不触碰用户文件
#[tauri::command]
pub fn container_move_item(app: AppHandle, path: String, x: i32, y: i32) -> Result<(), String> {
    let mut store = load_store(&app);
    if let Some(entry) = store.layout.iter_mut().find(|e| e.path == path) {
        entry.x = x;
        entry.y = y;
    } else {
        store.layout.push(LayoutEntry { path, x, y });
    }
    save_store(&app, &store)
}

/// 批量保存布局（拖拽结束后一次落盘，减少 IO）
#[tauri::command]
pub fn container_save_layout(app: AppHandle, entries: Vec<LayoutEntry>) -> Result<(), String> {
    let mut store = load_store(&app);
    for e in entries {
        if let Some(existing) = store.layout.iter_mut().find(|x| x.path == e.path) {
            existing.x = e.x;
            existing.y = e.y;
        } else {
            store.layout.push(e);
        }
    }
    save_store(&app, &store)
}

#[tauri::command]
pub fn container_load_layout(app: AppHandle) -> Vec<LayoutEntry> {
    load_store(&app).layout
}

/// 打开项目（opener 插件：文件用默认程序、目录用资源管理器）
#[tauri::command]
pub fn container_open_item(app: AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

/// 在资源管理器中定位
#[tauri::command]
pub fn container_reveal_item(app: AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|e| e.to_string())
}

/// 虚拟分组 CRUD
#[tauri::command]
pub fn group_list(app: AppHandle) -> Vec<GroupEntry> {
    load_store(&app).groups
}

#[tauri::command]
pub fn group_create(app: AppHandle, name: String) -> Result<Vec<GroupEntry>, String> {
    let mut store = load_store(&app);
    let id = format!("g{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0));
    store.groups.push(GroupEntry { id, name, item_paths: Vec::new() });
    save_store(&app, &store)?;
    Ok(store.groups)
}

#[tauri::command]
pub fn group_add_item(app: AppHandle, group_id: String, path: String) -> Result<(), String> {
    let mut store = load_store(&app);
    let g = store
        .groups
        .iter_mut()
        .find(|g| g.id == group_id)
        .ok_or("group not found")?;
    if !g.item_paths.contains(&path) {
        g.item_paths.push(path);
    }
    save_store(&app, &store)
}

#[tauri::command]
pub fn group_remove_item(app: AppHandle, group_id: String, path: String) -> Result<(), String> {
    let mut store = load_store(&app);
    let g = store
        .groups
        .iter_mut()
        .find(|g| g.id == group_id)
        .ok_or("group not found")?;
    g.item_paths.retain(|p| *p != path);
    save_store(&app, &store)
}

// ---- 图标提取（shell 图标 → 32x32 PNG base64） ----

#[tauri::command]
pub fn container_icon(path: String) -> Result<String, String> {
    let r = icon_png_base64(&path);
    match &r {
        Ok(b64) => log::info!("icon ok: {path} len={}", b64.len()),
        Err(e) => log::warn!("icon fail: {path}: {e}"),
    }
    r
}

fn icon_png_base64(path: &str) -> Result<String, String> {
    unsafe {
        let wide: Vec<u16> = path.encode_utf16().chain([0]).collect();
        let mut info = SHFILEINFOW::default();
        // 返回 0 表示失败（成功返回 hIcon 相关标志位；用 SHGFI_ICON 时非零即可）
        let r = SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_SMALLICON,
        );
        if r == 0 || info.hIcon.0.is_null() {
            log::warn!("icon: SHGetFileInfoW failed for {path}");
            return Err("SHGetFileInfoW failed".into());
        }
        let hicon = windows::Win32::UI::WindowsAndMessaging::HICON(info.hIcon.0);

        const S: usize = 32;
        let mut header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: S as i32,
            biHeight: S as i32, // bottom-up（GDI 绘制不支持 top-down DIB——实测画不上去）
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0, // BI_RGB
            ..Default::default()
        };
        let bmi = BITMAPINFO {
            bmiHeader: header,
            bmiColors: [Default::default(); 1],
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hdc = GetDC(None);
        let memdc = CreateCompatibleDC(Some(hdc));
        let hbm = CreateDIBSection(Some(memdc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
            .map_err(|e| format!("CreateDIBSection: {e}"))?;
        // 清底（DIB 初始内容未定义，防垃圾背景）
        std::ptr::write_bytes(bits as *mut u8, 0, S * S * 4);
        let old = SelectObject(memdc, hbm.into());
        // 清底 + 绘制图标
        DrawIconEx(
            memdc,
            0,
            0,
            hicon,
            S as i32,
            S as i32,
            0,
            None,
            DI_NORMAL,
        );
        // 注意：DIB section 是直写缓冲，DrawIconEx 后 bits 已含内容；
        // 不要调用 GetDIBits 读回（实测会清零 DIB 自身缓冲——未定义行为）
        // BGRA(bottom-up) → RGBA(top-down)：按行从底到顶翻转
        let raw = std::slice::from_raw_parts(bits as *const u8, S * S * 4);
        let mut rgba = Vec::with_capacity(S * S * 4);
        for row in (0..S).rev() {
            let start = row * S * 4;
            for px in raw[start..start + S * 4].chunks_exact(4) {
                rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
            }
        }
        SelectObject(memdc, old);
        DeleteObject(hbm.into());
        DeleteDC(memdc);

        let img = image::RgbaImage::from_raw(S as u32, S as u32, rgba)
            .ok_or_else(|| {
                log::warn!("icon: RgbaImage::from_raw failed for {path}");
                "RgbaImage::from_raw failed".to_string()
            })?;
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        Ok(base64::engine::general_purpose::STANDARD.encode(out.into_inner()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static SEQ: AtomicU32 = AtomicU32::new(0);

    fn temp_store_path(name: &str) -> PathBuf {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("lumina-store-{name}-{n}.json"))
    }

    fn entry(path: &str, x: i32, y: i32) -> LayoutEntry {
        LayoutEntry { path: path.into(), x, y }
    }

    #[test]
    fn store_roundtrip() {
        let p = temp_store_path("rt");
        let mut s = ContainerStore::default();
        s.layout.push(entry("a", 10, 20));
        s.groups.push(GroupEntry { id: "g1".into(), name: "组".into(), item_paths: vec!["a".into()] });
        save_store_at(&p, &s).unwrap();
        let loaded = load_store_at(&p);
        assert_eq!(loaded.layout.len(), 1);
        assert_eq!(loaded.layout[0].x, 10);
        assert_eq!(loaded.layout[0].y, 20);
        assert_eq!(loaded.groups.len(), 1);
        assert_eq!(loaded.groups[0].item_paths, vec!["a".to_string()]);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn store_corruption_falls_back_to_bak() {
        let p = temp_store_path("corrupt");
        let s1 = { let mut s = ContainerStore::default(); s.layout.push(entry("a", 1, 1)); s };
        save_store_at(&p, &s1).unwrap();
        let s2 = { let mut s = ContainerStore::default(); s.layout.push(entry("a", 2, 2)); s };
        save_store_at(&p, &s2).unwrap(); // 覆盖前留 .bak（第一版）
        fs::write(&p, b"{{{garbage").unwrap();
        let loaded = load_store_at(&p);
        assert_eq!(loaded.layout[0].x, 1, "应回退到 .bak 的第一版");
        let _ = fs::remove_file(&p);
        let _ = fs::remove_file(format!("{}.bak", p.display()));
    }

    #[test]
    fn move_overwrites_same_path() {
        // 模拟 container_move_item 的合并语义
        let p = temp_store_path("move");
        let mut s = ContainerStore::default();
        s.layout.push(entry("a", 1, 1));
        save_store_at(&p, &s).unwrap();

        let mut s2 = load_store_at(&p);
        if let Some(e) = s2.layout.iter_mut().find(|e| e.path == "a") {
            e.x = 99; e.y = 88;
        }
        save_store_at(&p, &s2).unwrap();

        let loaded = load_store_at(&p);
        assert_eq!(loaded.layout.len(), 1, "同路径覆盖而非追加");
        assert_eq!((loaded.layout[0].x, loaded.layout[0].y), (99, 88));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn save_layout_merges_batch() {
        // 模拟 container_save_layout 的批量合并语义（新旧混合）
        let p = temp_store_path("batch");
        let mut s = ContainerStore::default();
        s.layout.push(entry("old", 5, 5));
        save_store_at(&p, &s).unwrap();

        let mut s2 = load_store_at(&p);
        for e in [entry("old", 7, 7), entry("new", 3, 3)] {
            if let Some(x) = s2.layout.iter_mut().find(|x| x.path == e.path) {
                x.x = e.x; x.y = e.y;
            } else {
                s2.layout.push(e);
            }
        }
        save_store_at(&p, &s2).unwrap();

        let loaded = load_store_at(&p);
        assert_eq!(loaded.layout.len(), 2);
        let old = loaded.layout.iter().find(|e| e.path == "old").unwrap();
        assert_eq!((old.x, old.y), (7, 7));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn group_crud() {
        let p = temp_store_path("group");
        let mut s = ContainerStore::default();
        s.groups.push(GroupEntry { id: "g1".into(), name: "G".into(), item_paths: vec![] });
        save_store_at(&p, &s).unwrap();

        // add
        let mut s2 = load_store_at(&p);
        let g = s2.groups.iter_mut().find(|g| g.id == "g1").unwrap();
        g.item_paths.push("x".into());
        save_store_at(&p, &s2).unwrap();
        assert_eq!(load_store_at(&p).groups[0].item_paths, vec!["x".to_string()]);

        // remove
        let mut s3 = load_store_at(&p);
        let g = s3.groups.iter_mut().find(|g| g.id == "g1").unwrap();
        g.item_paths.retain(|i| i != "x");
        save_store_at(&p, &s3).unwrap();
        assert!(load_store_at(&p).groups[0].item_paths.is_empty());
        let _ = fs::remove_file(&p);
    }

}
