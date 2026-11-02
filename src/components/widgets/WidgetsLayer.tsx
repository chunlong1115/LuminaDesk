import type { ComponentType } from "react";
import { useWidgets, type WidgetKind } from "../../stores/widgets";
import { Clock } from "./Clock";
import { CpuMem } from "./CpuMem";
import { DateWidget } from "./Date";
import { Notes } from "./Notes";
import { Todo } from "./Todo";
import styles from "./WidgetsLayer.module.css";

/** 组件 → 渲染器映射（便签坐标独立于卡片位，特殊处理） */
const RENDERERS: Record<WidgetKind, ComponentType> = {
  clock: Clock,
  date: DateWidget,
  cpuMem: CpuMem,
  notes: Notes,
  todo: Todo,
};

/**
 * 小组件层：默认合并进容器窗口（壁纸层，图标之上）。
 * 层本身不拦截鼠标（pointer-events:none），卡片/便签各自接收事件，
 * 空白区照常框选/拖动图标。「浮于图标之上」模式见 widgets 窗口（阶段 3 任务 3）。
 */
export function WidgetsLayer() {
  const widgets = useWidgets((s) => s.widgets);
  return (
    <div className={styles.layer}>
      {widgets.map((w) => {
        const Cmp = RENDERERS[w.kind];
        if (w.kind === "notes") {
          // 便签卡片坐标取自身（容器层坐标系），仅占位不套卡片
          return <Cmp key={w.id} />;
        }
        return (
          <div
            key={w.id}
            className={`${styles.card} ${w.kind === "todo" ? styles.cardTodo : ""}`}
            style={{ left: w.x, top: w.y }}
          >
            <Cmp />
          </div>
        );
      })}
    </div>
  );
}
