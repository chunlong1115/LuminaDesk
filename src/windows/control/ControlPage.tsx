import { useCallback, useEffect, useState } from "react";
import { layerAttachNow, layerDetachNow, layerStatus } from "../../lib/ipc";
import { useSettings } from "../../stores/settings";
import styles from "./ControlPage.module.css";

/** 主控制窗口（设置、容器/组件管理、文件整理；关闭即销毁） */
export function ControlPage() {
  const [status, setStatus] = useState("检测中…");
  const [busy, setBusy] = useState(false);
  const { desktopLayerEnabled, containerOpacity, set } = useSettings();

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
    </div>
  );
}
