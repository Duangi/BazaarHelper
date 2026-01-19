import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import OverlayApp from "./OverlayApp";
import { getCurrentWindow } from "@tauri-apps/api/window";

function Root() {
  const [label, setLabel] = useState<string | null>(null);

  useEffect(() => {
    setLabel(getCurrentWindow().label);
  }, []);

  if (label === null) return null;

  return (
    <React.StrictMode>
      {label === "overlay" ? <OverlayApp /> : <App />}
    </React.StrictMode>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(<Root />);
