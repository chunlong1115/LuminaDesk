//! 组件管理器持久化与 CRUD 命令（widgets.json）。
//! 写路径：invoke → 内存修改 → atomic_write 落盘（损坏回退 .bak）→ emit 广播
//! （管理页改配置 → 容器/浮层窗口重渲染）。便签/待办为单窗口编辑，不广播。
//! 纯函数（默认布局/校验/增删）下沉便于单测，命令层只做读写与分发。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::persistence::{atomic_write, load_with_fallback};

/// 组件配置（kind 与前端 WidgetKind 对齐；x/y 为容器逻辑坐标）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WidgetConfig {
    pub id: String,
    pub kind: String,
    pub x: i32,
    pub y: i32,
}

/// 便签（w/h 为逻辑尺寸，缩放范围由前端 clampNoteSize 钳制）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoteItem {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub text: String,
}

/// 待办
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TodoItem {
    pub id: String,
    pub text: String,
    pub done: bool,
}

/// 组件持久化数据（widgets.json）
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WidgetStore {
    pub widgets: Vec<WidgetConfig>,
    pub notes: Vec<NoteItem>,
    pub todos: Vec<TodoItem>,
}

/// widgets 配置变更广播（跨窗口：管理页改 → 容器/浮层重渲染）
pub const WIDGETS_CHANGED: &str = "lumina://widgets-changed";

/// 合法组件类型（与前端 WidgetKind 对齐）
pub const VALID_KINDS: [&str; 5] = ["clock", "date", "cpuMem", "notes", "todo"];

/// 默认五组件布局（与前端 DEFAULT_WIDGETS 对齐；文件缺失时回退）
pub(crate) fn default_widgets() -> Vec<WidgetConfig> {
    [
        ("clock", 1200, 28),
        ("date", 1200, 106),
        ("cpuMem", 1200, 170),
        ("notes", 1020, 340),
        ("todo", 1256, 340),
    ]
    .into_iter()
    .map(|(kind, x, y)| WidgetConfig {
        id: kind.to_string(),
        kind: kind.to_string(),
        x,
        y,
    })
    .collect()
}

/// 组件类型校验
pub(crate) fn is_valid_kind(kind: &str) -> bool {
    VALID_KINDS.contains(&kind)
}

// ---- 纯函数：列表增删（命令层复用，可单测） ----

pub(crate) fn upsert_widget(list: &[WidgetConfig], cfg: &WidgetConfig) -> Vec<WidgetConfig> {
    let mut out = list.to_vec();
    match out.iter_mut().find(|w| w.id == cfg.id) {
        Some(slot) => *slot = cfg.clone(),
        None => out.push(cfg.clone()),
    }
    out
}

pub(crate) fn remove_widget(list: &[WidgetConfig], id: &str) -> Vec<WidgetConfig> {
    list.iter().filter(|w| w.id != id).cloned().collect()
}

pub(crate) fn upsert_note(list: &[NoteItem], note: &NoteItem) -> Vec<NoteItem> {
    let mut out = list.to_vec();
    match out.iter_mut().find(|n| n.id == note.id) {
        Some(slot) => *slot = note.clone(),
        None => out.push(note.clone()),
    }
    out
}

pub(crate) fn remove_note(list: &[NoteItem], id: &str) -> Vec<NoteItem> {
    list.iter().filter(|n| n.id != id).cloned().collect()
}

pub(crate) fn upsert_todo(list: &[TodoItem], todo: &TodoItem) -> Vec<TodoItem> {
    let mut out = list.to_vec();
    match out.iter_mut().find(|t| t.id == todo.id) {
        Some(slot) => *slot = todo.clone(),
        None => out.push(todo.clone()),
    }
    out
}

pub(crate) fn remove_todo(list: &[TodoItem], id: &str) -> Vec<TodoItem> {
    list.iter().filter(|t| t.id != id).cloned().collect()
}

// ---- 内部工具 ----

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|d| d.join("widgets.json"))
        .map_err(|e| e.to_string())
}

/// 纯函数：读取（缺失/损坏 → None，默认值由调用方决定）——可单测
pub(crate) fn load_store_at(path: &Path) -> Option<WidgetStore> {
    load_with_fallback(path)
}

/// 纯函数：原子写入——可单测
pub(crate) fn save_store_at(path: &Path, store: &WidgetStore) -> Result<(), String> {
    atomic_write(path, store).map_err(|e| e.to_string())
}

/// 文件缺失/损坏 → 默认五组件 + 空便签/待办（组件配置的语义默认）
pub(crate) fn load_or_default(path: &Path) -> WidgetStore {
    load_store_at(path).unwrap_or_else(|| WidgetStore {
        widgets: default_widgets(),
        ..Default::default()
    })
}

// ---- 组件配置命令 ----

#[tauri::command]
pub fn widgets_list(app: AppHandle) -> Vec<WidgetConfig> {
    store_path(&app)
        .ok()
        .map(|p| load_or_default(&p).widgets)
        .unwrap_or_else(default_widgets)
}

#[tauri::command]
pub fn widgets_upsert(app: AppHandle, config: WidgetConfig) -> Result<Vec<WidgetConfig>, String> {
    if !is_valid_kind(&config.kind) {
        return Err(format!("unknown widget kind: {}", config.kind));
    }
    let p = store_path(&app)?;
    let mut store = load_or_default(&p);
    store.widgets = upsert_widget(&store.widgets, &config);
    save_store_at(&p, &store)?;
    let widgets = store.widgets;
    let _ = app.emit(WIDGETS_CHANGED, widgets.clone());
    Ok(widgets)
}

#[tauri::command]
pub fn widgets_remove(app: AppHandle, id: String) -> Result<Vec<WidgetConfig>, String> {
    let p = store_path(&app)?;
    let mut store = load_or_default(&p);
    store.widgets = remove_widget(&store.widgets, &id);
    save_store_at(&p, &store)?;
    let widgets = store.widgets;
    let _ = app.emit(WIDGETS_CHANGED, widgets.clone());
    Ok(widgets)
}

// ---- 便签命令（单窗口编辑，不广播） ----

#[tauri::command]
pub fn notes_list(app: AppHandle) -> Vec<NoteItem> {
    store_path(&app)
        .ok()
        .and_then(|p| load_store_at(&p))
        .map(|s| s.notes)
        .unwrap_or_default()
}

#[tauri::command]
pub fn notes_upsert(app: AppHandle, note: NoteItem) -> Result<Vec<NoteItem>, String> {
    let p = store_path(&app)?;
    let mut store = load_store_at(&p).unwrap_or_default();
    store.notes = upsert_note(&store.notes, &note);
    save_store_at(&p, &store)?;
    Ok(store.notes)
}

#[tauri::command]
pub fn notes_remove(app: AppHandle, id: String) -> Result<Vec<NoteItem>, String> {
    let p = store_path(&app)?;
    let mut store = load_store_at(&p).unwrap_or_default();
    store.notes = remove_note(&store.notes, &id);
    save_store_at(&p, &store)?;
    Ok(store.notes)
}

// ---- 待办命令（单窗口编辑，不广播） ----

#[tauri::command]
pub fn todos_list(app: AppHandle) -> Vec<TodoItem> {
    store_path(&app)
        .ok()
        .and_then(|p| load_store_at(&p))
        .map(|s| s.todos)
        .unwrap_or_default()
}

#[tauri::command]
pub fn todos_upsert(app: AppHandle, todo: TodoItem) -> Result<Vec<TodoItem>, String> {
    let p = store_path(&app)?;
    let mut store = load_store_at(&p).unwrap_or_default();
    store.todos = upsert_todo(&store.todos, &todo);
    save_store_at(&p, &store)?;
    Ok(store.todos)
}

#[tauri::command]
pub fn todos_remove(app: AppHandle, id: String) -> Result<Vec<TodoItem>, String> {
    let p = store_path(&app)?;
    let mut store = load_store_at(&p).unwrap_or_default();
    store.todos = remove_todo(&store.todos, &id);
    save_store_at(&p, &store)?;
    Ok(store.todos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static SEQ: AtomicU32 = AtomicU32::new(0);

    /// 每个测试独立的子目录（测试并行运行，禁止共享目录）。
    /// 目录名确定性复用 → 先清上次异常中断的残留，再创建
    fn unique_dir(name: &str) -> PathBuf {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let d = std::env::temp_dir().join(format!("lumina-widgets-{name}-{n}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn cfg(kind: &str, x: i32, y: i32) -> WidgetConfig {
        WidgetConfig { id: kind.to_string(), kind: kind.to_string(), x, y }
    }

    fn note(id: &str, text: &str) -> NoteItem {
        NoteItem { id: id.to_string(), x: 0, y: 0, w: 160, h: 120, text: text.to_string() }
    }

    fn todo(id: &str, text: &str) -> TodoItem {
        TodoItem { id: id.to_string(), text: text.to_string(), done: false }
    }

    #[test]
    fn default_widgets_shape() {
        let ws = default_widgets();
        assert_eq!(ws.len(), 5);
        assert_eq!(ws[0], cfg("clock", 1200, 28));
        assert_eq!(ws[1], cfg("date", 1200, 106));
        assert_eq!(ws[2], cfg("cpuMem", 1200, 170));
        assert_eq!(ws[3], cfg("notes", 1020, 340));
        assert_eq!(ws[4], cfg("todo", 1256, 340));
        // id 与 kind 一致（默认组件）
        assert!(ws.iter().all(|w| w.id == w.kind));
    }

    #[test]
    fn is_valid_kind_cases() {
        for k in VALID_KINDS {
            assert!(is_valid_kind(k));
        }
        assert!(!is_valid_kind("weather"));
        assert!(!is_valid_kind(""));
        assert!(!is_valid_kind("Clock")); // 大小写敏感
    }

    #[test]
    fn upsert_widget_insert_and_replace() {
        let list = vec![cfg("clock", 1200, 28)];
        // 新增
        let out = upsert_widget(&list, &cfg("date", 1200, 106));
        assert_eq!(out.len(), 2);
        // 同 id 替换位置
        let out = upsert_widget(&out, &cfg("clock", 100, 50));
        assert_eq!(out.len(), 2);
        assert!(out.iter().any(|w| w.id == "clock" && w.x == 100 && w.y == 50));
    }

    #[test]
    fn remove_widget_missing_id_noop() {
        let list = vec![cfg("clock", 1200, 28), cfg("date", 1200, 106)];
        assert_eq!(remove_widget(&list, "clock").len(), 1);
        assert_eq!(remove_widget(&list, "nope"), list); // 不存在 → 原样
    }

    #[test]
    fn upsert_note_insert_and_replace() {
        let list = vec![note("n1", "a")];
        let out = upsert_note(&list, &note("n2", "b"));
        assert_eq!(out.len(), 2);
        let moved = NoteItem { id: "n1".into(), x: 5, y: 6, w: 200, h: 150, text: "a".into() };
        let out = upsert_note(&out, &moved);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], moved);
    }

    #[test]
    fn remove_note_and_todo() {
        let notes = vec![note("n1", "a"), note("n2", "b")];
        assert_eq!(remove_note(&notes, "n2"), vec![note("n1", "a")]);
        let todos = vec![todo("t1", "x"), todo("t2", "y")];
        assert_eq!(remove_todo(&todos, "t1"), vec![todo("t2", "y")]);
    }

    #[test]
    fn upsert_todo_toggle_done() {
        let list = vec![todo("t1", "x")];
        let done = TodoItem { id: "t1".into(), text: "x".into(), done: true };
        let out = upsert_todo(&list, &done);
        assert_eq!(out.len(), 1);
        assert!(out[0].done);
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = unique_dir("roundtrip");
        let path = dir.join("widgets.json");
        let store = WidgetStore {
            widgets: vec![cfg("clock", 1, 2)],
            notes: vec![note("n1", "hi")],
            todos: vec![todo("t1", "buy milk")],
        };
        save_store_at(&path, &store).unwrap();
        assert_eq!(load_store_at(&path), Some(store));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_missing_or_corrupt() {
        let dir = unique_dir("corrupt");
        // 缺失 → None；load_or_default 回退默认五组件
        let path = dir.join("widgets.json");
        assert!(load_store_at(&path).is_none());
        assert_eq!(load_or_default(&path).widgets, default_widgets());
        // 损坏 → None（.bak 也缺失）
        std::fs::write(&path, b"{ not json").unwrap();
        assert!(load_store_at(&path).is_none());
        // 损坏但有 .bak → 回退 .bak
        let bak = dir.join("widgets.json.bak");
        save_store_at(&bak, &WidgetStore { widgets: vec![cfg("date", 9, 9)], ..Default::default() }).unwrap();
        let store = load_store_at(&path).unwrap();
        assert_eq!(store.widgets, vec![cfg("date", 9, 9)]);
        std::fs::remove_dir_all(&dir).ok();
    }
}
