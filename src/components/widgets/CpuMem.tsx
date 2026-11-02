import { useEffect } from "react";
import { formatMb } from "../../lib/format";
import { useStats } from "../../stores/stats";
import { Sparkline } from "../common/Sparkline";
import styles from "./widgets.module.css";

/** CPU/内存组件：60 点环形缓冲 + SVG 迷你图（Rust 侧 1Hz 推送，前端零轮询） */
export function CpuMem() {
  const history = useStats((s) => s.history);
  const subscribe = useStats((s) => s.subscribe);
  const unsubscribe = useStats((s) => s.unsubscribe);

  useEffect(() => {
    subscribe();
    return unsubscribe;
  }, [subscribe, unsubscribe]);

  const last = history[history.length - 1];
  const cpu = last ? `${Math.round(last.cpu)}%` : "--";
  const mem = last ? `${formatMb(last.memUsedMb)} / ${formatMb(last.memTotalMb)}` : "--";

  return (
    <div className={styles.cpuMem}>
      <div className={styles.metricRow}>
        <span className={styles.metricLabel}>CPU</span>
        <span className={styles.metricValue}>{cpu}</span>
      </div>
      <Sparkline data={history.map((s) => s.cpu)} />
      <div className={styles.metricRow}>
        <span className={styles.metricLabel}>内存</span>
        <span className={styles.metricValue}>{mem}</span>
      </div>
      <Sparkline data={history.map((s) => s.memUsedMb)} />
    </div>
  );
}
