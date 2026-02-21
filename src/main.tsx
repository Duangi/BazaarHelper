import { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import YoloMonitor from "./YoloMonitor";
import DetailPopup from "./DetailPopup";
import MonsterCalibration from "./MonsterCalibration";
import { getCurrentWindow } from "@tauri-apps/api/window";

function Root() {
  const [component, setComponent] = useState<string | null>(null);

  useEffect(() => {
    const windowLabel = getCurrentWindow().label;
    const windowType = (window as any).__WINDOW_TYPE__;
    
    console.log("[Main.tsx] Window label:", windowLabel);
    console.log("[Main.tsx] Window type from global:", windowType);
    console.log("[Main.tsx] Full URL:", window.location.href);
    
    // 优先使用全局变量，其次使用 window label
    const componentType = windowType || windowLabel;
    console.log("[Main.tsx] Component type:", componentType);
    setComponent(componentType);
  }, []);

  useEffect(() => {
    const windowLabel = getCurrentWindow().label;
    const perfWithMemory = performance as Performance & {
      memory?: {
        usedJSHeapSize: number;
        totalJSHeapSize: number;
        jsHeapSizeLimit: number;
      };
    };

    if (!perfWithMemory.memory) return;

    const toMB = (v: number) => (v / 1024 / 1024).toFixed(1);
    const timer = window.setInterval(() => {
      const m = perfWithMemory.memory;
      if (!m) return;
      console.debug(
        `[WebMem][${windowLabel}] used=${toMB(m.usedJSHeapSize)}MB total=${toMB(m.totalJSHeapSize)}MB limit=${toMB(m.jsHeapSizeLimit)}MB`,
      );
    }, 10000);

    return () => window.clearInterval(timer);
  }, []);

  if (component === null) {
    console.log("[Main.tsx] Component is null, waiting...");
    return null;
  }

  console.log("[Main.tsx] Rendering component for:", component);
  
  if (component === "yolo-monitor") {
    console.log("[Main.tsx] ✅ Rendering YoloMonitor");
    return <YoloMonitor />;
  } else if (component === "detail-popup") {
    console.log("[Main.tsx] ✅ Rendering DetailPopup");
    return <DetailPopup />;
  } else if (component === "monster-calibration") {
    console.log("[Main.tsx] ✅ Rendering MonsterCalibration");
    return <MonsterCalibration />;
  } else {
    console.log("[Main.tsx] ✅ Rendering App (main window)");
    return <App />;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(<Root />);
