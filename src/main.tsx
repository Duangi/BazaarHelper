import React, { useEffect, useState } from "react";
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

  if (component === null) {
    console.log("[Main.tsx] Component is null, waiting...");
    return null;
  }

  console.log("[Main.tsx] Rendering component for:", component);
  
  if (component === "yolo-monitor") {
    console.log("[Main.tsx] ✅ Rendering YoloMonitor");
    return <React.StrictMode><YoloMonitor /></React.StrictMode>;
  } else if (component === "detail-popup") {
    console.log("[Main.tsx] ✅ Rendering DetailPopup");
    return <React.StrictMode><DetailPopup /></React.StrictMode>;
  } else if (component === "monster-calibration") {
    console.log("[Main.tsx] ✅ Rendering MonsterCalibration");
    return <React.StrictMode><MonsterCalibration /></React.StrictMode>;
  } else {
    console.log("[Main.tsx] ✅ Rendering App (main window)");
    return <React.StrictMode><App /></React.StrictMode>;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(<Root />);
