import { useEffect } from "react";
import { WaveMap } from "../components/waves/WaveMap";
import { useConfigStore } from "../stores/configStore";
import { useGameStore } from "../stores/gameStore";
import { useWaveStore } from "../stores/waveStore";

/**
 * The click-through overlay view, rendered in its own borderless Tauri window
 * positioned over Dota's in-game minimap.
 *
 * Deliberately chrome-free: no background, no border, no text. Anything opaque
 * here would hide the real minimap underneath.
 */
export default function WaveOverlay() {
  const lanePaths = useWaveStore((s) => s.lanePaths);
  const snapshot = useWaveStore((s) => s.snapshot);
  const startTracking = useWaveStore((s) => s.startTracking);
  const opacity = useConfigStore((s) => s.config.wave_overlay.opacity);

  useEffect(() => {
    useConfigStore.getState().loadConfig();
    const gameUnlisten = useGameStore.getState().startListening();
    // This window is built once and then only hidden and re-shown, so without a
    // live subscription it would render the config as it stood the first time the
    // overlay was opened.
    const configUnlisten = useConfigStore.getState().startListening();
    const stopTracking = startTracking();

    return () => {
      stopTracking();
      gameUnlisten.then((unlisten) => unlisten());
      configUnlisten.then((unlisten) => unlisten());
    };
  }, [startTracking]);

  return (
    <div
      className="h-screen w-screen"
      style={{ background: "transparent", opacity }}
    >
      <WaveMap lanePaths={lanePaths} snapshot={snapshot} compact />
    </div>
  );
}
