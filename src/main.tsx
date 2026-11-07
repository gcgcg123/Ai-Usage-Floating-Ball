import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import SettingsApp from "./SettingsApp";

function isSettingsWindow() {
  const queryMode =
    new URLSearchParams(window.location.search).get("window") === "settings";
  if (queryMode) return true;
  try {
    return getCurrentWindow().label === "settings";
  } catch {
    return false;
  }
}

const settingsMode = isSettingsWindow();
if (settingsMode) {
  document.documentElement.classList.add("settings-window");
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {settingsMode ? <SettingsApp /> : <App />}
  </React.StrictMode>,
);
