import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import LatchIndicator from "./pages/LatchIndicator";
import WaveOverlay from "./pages/WaveOverlay";
import { applyOverlayBodyStyles, overlayKind } from "./lib/overlay";
import "./styles/global.css";

const overlay = overlayKind();

// Before first paint: an opaque body would hide the game underneath.
if (overlay) {
  applyOverlayBodyStyles();
}

const root =
  overlay === "latch" ? <LatchIndicator /> : overlay === "wave" ? <WaveOverlay /> : <App />;

createRoot(document.getElementById("root")!).render(<StrictMode>{root}</StrictMode>);
