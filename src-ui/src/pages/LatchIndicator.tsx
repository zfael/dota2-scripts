import { useEffect, useState } from "react";
import { isTauri } from "../lib/tauri";

/** Emitted by `src-tauri/src/commands/latch_indicator.rs`; `true` is on. */
const LATCH_EVENT = "pl_latch_update";

/**
 * Backstop read. The event is what makes the box change with the cue; this
 * only keeps a lost event from leaving it wrong for the rest of the game.
 */
const RESYNC_MS = 500;

/**
 * Phantom Lancer's latch indicator, rendered in its own click-through window
 * over Dota: green while the Ctrl latch is on, red while it is off.
 *
 * Rust decides when the window is shown and where; this view only paints the
 * state it is sent. The window is sized from Dota's height, so the text is
 * sized from the window's.
 */
export default function LatchIndicator() {
  const [latched, setLatched] = useState(false);

  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    let resync: ReturnType<typeof setInterval> | undefined;

    (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      const { invoke } = await import("@tauri-apps/api/core");
      const read = async () => {
        try {
          const current = await invoke<boolean>("get_pl_latch_state");
          if (!cancelled) setLatched(current);
        } catch (e) {
          console.error("Failed to read the latch state:", e);
        }
      };
      // Listen first, then ask: a change in between is caught either way.
      const stop = await listen<boolean>(LATCH_EVENT, (event) => setLatched(event.payload));
      if (cancelled) {
        stop();
        return;
      }
      unlisten = stop;
      await read();
      if (!cancelled) resync = setInterval(read, RESYNC_MS);
    })();

    return () => {
      cancelled = true;
      unlisten?.();
      if (resync) clearInterval(resync);
    };
  }, []);

  return (
    <div
      className="flex h-screen w-screen select-none items-center justify-center rounded-md font-bold tracking-wider text-white"
      style={{
        background: latched ? "rgba(22, 163, 74, 0.9)" : "rgba(220, 38, 38, 0.9)",
        border: "2px solid rgba(0, 0, 0, 0.55)",
        fontSize: "48vh",
        lineHeight: 1,
        textShadow: "0 1px 2px rgba(0, 0, 0, 0.6)",
      }}
    >
      {latched ? "LATCH ON" : "LATCH OFF"}
    </div>
  );
}
