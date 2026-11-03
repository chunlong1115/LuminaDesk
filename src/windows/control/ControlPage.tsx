import { useCallback, useEffect, useState } from "react";
import {
  desktopMenuGet,
  desktopMenuSet,
  layerAttachNow,
  layerDetachNow,
  layerStatus,
  systemSetWidgetsAboveIcons,
  widgetsList,
  widgetsModeGet,
  widgetsRemove,
  widgetsUpsert,
  type WidgetConfig,
  type WidgetKind,
} from "../../lib/ipc";
import { useSettings } from "../../stores/settings";
import { newWidgetId, nextWidgetPos, parsePos, WIDGET_KIND_LABELS } from "./widgetPanel";
import styles from "./ControlPage.module.css";

/** 组件行：类型 + X/Y 调位（失焦/回车提交，非法输入回退原值）+ 删除 */
function WidgetRow({
  w,
  onUpsert,
  onRemove,
}: {
  w: WidgetConfig;
  onUpsert: (config: WidgetConfig) => void;
  onRemove: (id: string) => void;
}) {
  const [x, setX] = useState(String(w.x));
  const [y, setY] = useState(String(w.y));

  const commit = (nx: string, ny: string) => {
    const px = parsePos(nx, w.x);
    const py = parsePos(ny, w.y);
    if (px === w.x && py === w.y) return;
    onUpsert({ ...w, x: px, y: py });
  };

  return (
    <div className={styles.widgetRow}>
      <span className={styles.widgetKind}>{WIDGET_KIND_LABELS[w.kind]}</span>
      <label className={styles.widgetField}>
        X
        <input
          value={x}
          onChange={(e) => setX(e.target.value)}
          onBlur={() => commit(x, y)}
          onKeyDown={(e) => {
            if (e.key === "Enter") commit(x, y);
          }}
        />
      </label>
      <label className={styles.widgetField}>
        Y
        <input
          value={y}
          onChange={(e) => setY(e.target.value)}
          onBlur={() => commit(x, y)}
          onKeyDown={(e) => {
            if (e.key === "Enter") commit(x, y);
          }}
        />
      </label>
      <button className={styles.widgetRemove} onClick={() => onRemove(w.id)}>
        删除
      </button>
    </div>
  );
}

/** 主控制窗口（设置、容器/组件管理、文件整理；关闭即销毁） */
export function ControlPage() {
  const [status, setStatus] = useState("检测中…");
  const [busy, setBusy] = useState(false);
  const [desktopMenuEnabled, setDesktopMenuEnabled] = useState(false);
  const [widgets, setWidgets] = useState<WidgetConfig[]>([]);
  const [aboveIcons, setAboveIcons] = useState(false);
  const [addKind, setAddKind] = useState<WidgetKind>("clock");
  const { desktopLayerEnabled, containerOpacity, set } = useSettings();

  useEffect(() => {
    void desktopMenuGet().then(setDesktopMenuEnabled).catch(() => {});
    void widgetsList().then(setWidgets).catch(() => {});
    void widgetsModeGet().then(setAboveIcons).catch(() => {});
  }, []);

  const toggleDesktopMenu = async (enabled: boolean) => {
    try {
      setDesktopMenuEnabled(await desktopMenuSet(enabled));
    } catch (e) {
      setStatus(`桌面右键设置失败: ${e}`);
    }
  };

  const toggleAboveIcons = async (enabled: boolean) => {
    try {
      const r = await systemSetWidgetsAboveIcons(enabled);
      setAboveIcons(r);
      useSettings.getState().set({ widgetsInContainer: !r });
    } catch (e) {
      setStatus(`组件模式切换失败: ${e}`);
    }
  };

  const upsertWidget = async (config: WidgetConfig) => {
    try {
      setWidgets(await widgetsUpsert(config));
    } catch (e) {
      setStatus(`组件保存失败: ${e}`);
    }
  };

  const removeWidget = async (id: string) => {
    try {
      setWidgets(await widgetsRemove(id));
    } catch (e) {
      setStatus(`组件删除失败: ${e}`);
    }
  };

  const addWidget = async () => {
    const pos = nextWidgetPos(widgets);
    await upsertWidget({ id: newWidgetId(addKind), kind: addKind, x: pos.x, y: pos.y });
  };

  const refresh = useCallback(async () => {
    try {
      setStatus(await layerStatus());
    } catch (e) {
      setStatus(`查询失败: ${e}`);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const toggleLayer = async (enable: boolean) => {
    setBusy(true);
    try {
      const r = enable ? await layerAttachNow() : await layerDetachNow();
      setStatus(`${r}\n${await layerStatus()}`);
      set({ desktopLayerEnabled: enable });
    } catch (e) {
      setStatus(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className={styles.page}>
      <h1>LuminaDesk</h1>
      <section className={styles.card}>
        <h2>桌面层</h2>
        <div className={styles.row}>
          <button disabled={busy} onClick={() => void toggleLayer(true)}>
            挂载
          </button>
          <button disabled={busy} onClick={() => void toggleLayer(false)}>
            卸载
          </button>
          <button disabled={busy} onClick={() => void refresh()}>
            刷新状态
          </button>
          <label className={styles.check}>
            <input
              type="checkbox"
              checked={desktopLayerEnabled}
              onChange={(e) => void toggleLayer(e.target.checked)}
            />
            启用桌面层
          </label>
        </div>
        <pre className={styles.status}>{status}</pre>
      </section>
      <section className={styles.card}>
        <h2>外观</h2>
        <label className={styles.slider}>
          容器不透明度 {Math.round(containerOpacity * 100)}%
          <input
            type="range"
            min={30}
            max={100}
            value={Math.round(containerOpacity * 100)}
            onChange={(e) => set({ containerOpacity: Number(e.target.value) / 100 })}
          />
        </label>
      </section>
      <section className={styles.card}>
        <h2>组件管理</h2>
        <label className={styles.check}>
          <input
            type="checkbox"
            checked={aboveIcons}
            onChange={(e) => void toggleAboveIcons(e.target.checked)}
          />
          浮于图标之上（独立悬浮层 + 空白区点击穿透；关闭则合并进桌面容器）
        </label>
        <div className={styles.widgetAddRow}>
          <select
            value={addKind}
            onChange={(e) => setAddKind(e.target.value as WidgetKind)}
          >
            {(Object.entries(WIDGET_KIND_LABELS) as [WidgetKind, string][]).map(
              ([k, label]) => (
                <option key={k} value={k}>
                  {label}
                </option>
              ),
            )}
          </select>
          <button onClick={() => void addWidget()}>添加组件</button>
        </div>
        <div className={styles.widgetList}>
          {widgets.map((w) => (
            <WidgetRow
              key={w.id}
              w={w}
              onUpsert={(c) => void upsertWidget(c)}
              onRemove={(id) => void removeWidget(id)}
            />
          ))}
        </div>
      </section>
      <section className={styles.card}>
        <h2>实验功能</h2>
        <label className={styles.check}>
          <input
            type="checkbox"
            checked={desktopMenuEnabled}
            onChange={(e) => void toggleDesktopMenu(e.target.checked)}
          />
          桌面右键替换（拦截桌面右键显示 LuminaDesk 菜单，替代系统默认菜单）
        </label>
      </section>
    </div>
  );
}
