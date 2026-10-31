import { useEffect, useState } from "react";
import { ping } from "../../lib/ipc";
import styles from "./ControlPage.module.css";

/** 主控制窗口（设置、容器/组件管理、文件整理；关闭即销毁） */
export function ControlPage() {
  const [backend, setBackend] = useState("检测中…");

  useEffect(() => {
    ping()
      .then((r) => setBackend(`后端在线（${r}）`))
      .catch(() => setBackend("后端不可达"));
  }, []);

  return (
    <div className={styles.page}>
      <h1>LuminaDesk</h1>
      <p className={styles.status}>{backend}</p>
      <section className={styles.card}>
        <h2>阶段 1 骨架</h2>
        <ul>
          <li>桌面层：阶段 2</li>
          <li>容器：阶段 2</li>
          <li>小组件：阶段 3</li>
          <li>文件整理（仅预览+虚拟分组）：阶段 4</li>
        </ul>
      </section>
    </div>
  );
}
