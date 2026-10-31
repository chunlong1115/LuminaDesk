import React from "react";
import ReactDOM from "react-dom/client";
import { ControlPage } from "./windows/control/ControlPage";
import { ContainerPage } from "./windows/container/ContainerPage";
import { WidgetsPage } from "./windows/widgets/WidgetsPage";

/**
 * 角色分发：所有窗口共用同一入口，按 URL 参数加载各自页面
 * （每渲染进程只加载自己角色的 chunk，后续动态 import 分包）
 */
function route(): React.ReactElement {
  const params = new URLSearchParams(window.location.search);
  switch (params.get("role")) {
    case "container":
      return <ContainerPage />;
    case "widgets":
      return <WidgetsPage />;
    default:
      return <ControlPage />;
  }
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>{route()}</React.StrictMode>,
);
