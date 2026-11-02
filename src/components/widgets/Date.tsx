import { useEffect, useState } from "react";
import { formatDate } from "../../lib/format";
import styles from "./widgets.module.css";

/** 日期组件：低频自检（60s），仅跨日时触发重渲染 */
export function DateWidget() {
  const [date, setDate] = useState(() => new Date());

  useEffect(() => {
    const id = window.setInterval(() => {
      setDate((d) => {
        const n = new Date();
        return n.getDate() !== d.getDate() ? n : d;
      });
    }, 60_000);
    return () => window.clearInterval(id);
  }, []);

  return <div className={styles.date}>{formatDate(date)}</div>;
}
