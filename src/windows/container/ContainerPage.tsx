import styles from "./ContainerPage.module.css";

/**
 * 桌面容器窗口（壁纸层、图标之下）。
 * 阶段 2 落地：真实桌面文件夹只读虚拟列表、拖拽摆放、虚拟分组、布局持久化。
 */
export function ContainerPage() {
  return (
    <div className={styles.page}>
      <div className={styles.panel}>LuminaDesk 容器（阶段 2 落地）</div>
    </div>
  );
}
