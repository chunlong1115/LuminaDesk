import { useRef } from "react";
import { setActiveIfWidgets } from "../../lib/windowRole";
import { clampNoteSize, NOTES_ORIGIN, useNotes, type NoteItem } from "../../stores/notes";
import styles from "./widgets.module.css";

type DragMode = "move" | "resize";

interface DragState {
  id: string;
  startX: number;
  startY: number;
  ox: number;
  oy: number;
  ow: number;
  oh: number;
  mode: DragMode;
}

function NoteCard({ note }: { note: NoteItem }) {
  const update = useNotes((s) => s.update);
  const remove = useNotes((s) => s.remove);
  // 拖拽状态存 ref（沿用容器拖拽经验：真实起点=按下时渲染位，勿读异步 state）
  const dragRef = useRef<DragState | null>(null);

  const onDown = (e: React.PointerEvent, mode: DragMode) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
    dragRef.current = {
      id: note.id,
      startX: e.clientX,
      startY: e.clientY,
      ox: note.x,
      oy: note.y,
      ow: note.w,
      oh: note.h,
      mode,
    };
  };

  const onMove = (e: React.PointerEvent) => {
    const d = dragRef.current;
    if (!d) return;
    if (d.mode === "move") {
      update(d.id, {
        x: d.ox + e.clientX - d.startX,
        y: d.oy + e.clientY - d.startY,
      });
    } else {
      update(d.id, clampNoteSize(
        d.ow + e.clientX - d.startX,
        d.oh + e.clientY - d.startY,
      ));
    }
  };

  const onUp = () => {
    dragRef.current = null;
  };

  return (
    <div
      className={styles.note}
      style={{ left: note.x, top: note.y, width: note.w, height: note.h }}
      data-hit-region=""
      onPointerDown={(e) => onDown(e, "move")}
      onPointerMove={onMove}
      onPointerUp={onUp}
    >
      <div className={styles.noteHeader}>
        <span>便签</span>
        <button
          className={styles.noteClose}
          title="删除便签"
          onPointerDown={(e) => e.stopPropagation()}
          onClick={() => remove(note.id)}
        >
          ×
        </button>
      </div>
      <textarea
        className={styles.noteText}
        value={note.text}
        placeholder="记点什么…"
        onChange={(e) => update(note.id, { text: e.target.value })}
        onPointerDown={(e) => e.stopPropagation()}
        onFocus={() => setActiveIfWidgets(window.location.search, true)}
        onBlur={() => setActiveIfWidgets(window.location.search, false)}
      />
      <div className={styles.noteResize} onPointerDown={(e) => onDown(e, "resize")} />
    </div>
  );
}

/** 便签组件：多张、可拖拽缩放；卡片坐标取便签自身（容器层坐标系） */
export function Notes() {
  const notes = useNotes((s) => s.notes);
  const add = useNotes((s) => s.add);
  return (
    <>
      {notes.map((n) => (
        <NoteCard key={n.id} note={n} />
      ))}
      <button
        className={styles.noteAdd}
        style={{ left: NOTES_ORIGIN.x, top: NOTES_ORIGIN.y - 34 }}
        data-hit-region=""
        onClick={add}
      >
        ＋ 新便签
      </button>
    </>
  );
}
