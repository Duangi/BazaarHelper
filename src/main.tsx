import { Suspense, lazy, useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";

const App = lazy(() => import("./App"));
const YoloMonitor = lazy(() => import("./YoloMonitor"));
const DetailPopup = lazy(() => import("./DetailPopup"));
const MonsterCalibration = lazy(() => import("./MonsterCalibration"));

(() => {
  try {
    const keepVerboseConsole = localStorage.getItem("bh-debug-console") === "1";
    if (!keepVerboseConsole) {
      const noop = () => {};
      console.log = noop;
      console.info = noop;
      console.debug = noop;
    }

    const memorySafeMode = localStorage.getItem("bh-memory-safe-mode");
    const enabled = memorySafeMode !== "0";
    if (enabled) {
      document.documentElement.classList.add("memory-safe-mode");
    } else {
      document.documentElement.classList.remove("memory-safe-mode");
    }

    // Dev-only guardrail: reduce compositor pressure while profiling memory in tauri dev.
    if (import.meta.env.DEV) {
      document.documentElement.classList.add("dev-memory-safe");
    } else {
      document.documentElement.classList.remove("dev-memory-safe");
    }
  } catch {
    // ignore localStorage access failures
  }
})();

function Root() {
  const [component, setComponent] = useState<string | null>(null);

  useEffect(() => {
    const windowLabel = getCurrentWindow().label;
    const windowType = (window as any).__WINDOW_TYPE__;

    // 优先使用全局变量，其次使用 window label
    const componentType = windowType || windowLabel;
    setComponent(componentType);
  }, []);

  if (component === null) {
    return null;
  }
  
  return (
    <Suspense fallback={null}>
      {component === "yolo-monitor" ? (
        <YoloMonitor />
      ) : component === "detail-popup" ? (
        <DetailPopup />
      ) : component === "monster-calibration" ? (
        <MonsterCalibration />
      ) : (
        <App />
      )}
    </Suspense>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(<Root />);
