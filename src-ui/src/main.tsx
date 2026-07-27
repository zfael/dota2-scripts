import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import WaveOverlay from "./pages/WaveOverlay";
import { applyOverlayBodyStyles, isOverlayWindow } from "./lib/overlay";
import "./styles/global.css";

const overlay = isOverlayWindow();

// Before first paint: an opaque body would hide the minimap underneath.
if (overlay) {
  applyOverlayBodyStyles();
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>{overlay ? <WaveOverlay /> : <App />}</StrictMode>,
);
