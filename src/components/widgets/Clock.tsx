import { useEffect, useState } from "react";
import { formatClock, msToNextSecond } from "../../lib/format";
import styles from "./widgets.module.css";

/**
 * 时钟组件：仅整秒触发重渲染。
 * 首帧 setTimeout 对齐秒沿（非 1000 盲等，避免相位漂移），此后 1s 间隔。
 */
export function Clock() {
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    let interval: number | undefined;
    const timeout = window.setTimeout(() => {
      setNow(new Date());
      interval = window.setInterval(() => setNow(new Date()), 1000);
    }, msToNextSecond(new Date()));
    return () => {
      window.clearTimeout(timeout);
      window.clearInterval(interval);
    };
  }, []);

  return <div className={styles.clock}>{formatClock(now)}</div>;
}
