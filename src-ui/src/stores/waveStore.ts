import { create } from "zustand";
import type { LanePath, WaveOverlayStatus, WaveSnapshot } from "../types/waves";
import { isTauri } from "../lib/tauri";
import { useGameStore } from "./gameStore";

/** ~15Hz. Fast enough for smooth dots, cheap enough to leave running. */
const TICK_INTERVAL_MS = 66;

/**
 * How long an outstanding snapshot call may block the next one.
 *
 * Bounded on purpose: an unbounded in-flight guard turns a single stuck call
 * into a permanently frozen map.
 */
export const IN_FLIGHT_TIMEOUT_MS = 2000;

/**
 * How far the local clock may run past the last value GSI reported.
 *
 * GSI is authoritative and ticks `gameTime` once per second, so a little
 * extrapolation keeps dots moving smoothly between packets. Capping it also gives
 * us pause handling for free: when the game is paused (or GSI stalls, or the
 * connection drops) `gameTime` stops changing, the cap is reached, and the clock
 * freezes rather than drifting away. The next packet re-anchors it.
 */
export const MAX_CLOCK_DRIFT_SECONDS = 1.5;

/**
 * Local clock estimate between GSI packets.
 *
 * Exported for testing — this is the whole of the pause-handling logic.
 */
export function interpolatedClock(
  lastGameTime: number,
  lastReceivedAtMs: number,
  nowMs: number,
  maxDriftSeconds: number = MAX_CLOCK_DRIFT_SECONDS,
): number {
  const elapsedSeconds = Math.max(0, (nowMs - lastReceivedAtMs) / 1000);
  return lastGameTime + Math.min(elapsedSeconds, maxDriftSeconds);
}

interface WaveStore {
  lanePaths: LanePath[];
  snapshot: WaveSnapshot | null;
  overlayStatus: WaveOverlayStatus | null;
  /** Last snapshot fetch failure, surfaced in the UI. */
  error: string | null;
  fetchLanePaths: () => Promise<void>;
  fetchOverlayStatus: () => Promise<void>;
  toggleOverlay: () => Promise<void>;
  startTracking: () => () => void;
}

export const useWaveStore = create<WaveStore>((set, get) => ({
  lanePaths: [],
  snapshot: null,
  overlayStatus: null,
  error: null,

  fetchOverlayStatus: async () => {
    if (!isTauri()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const overlayStatus = await invoke<WaveOverlayStatus>("get_wave_overlay_status");
      set({ overlayStatus });
    } catch (e) {
      console.error("Failed to fetch wave overlay status:", e);
    }
  },

  toggleOverlay: async () => {
    if (!isTauri()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke<boolean>("toggle_wave_overlay");
    } catch (e) {
      console.error("Failed to toggle wave overlay:", e);
    }
    await get().fetchOverlayStatus();
  },

  fetchLanePaths: async () => {
    if (!isTauri()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const lanePaths = await invoke<LanePath[]>("get_wave_lane_paths");
      set({ lanePaths });
    } catch (e) {
      console.error("Failed to fetch wave lane paths:", e);
    }
  },

  startTracking: () => {
    void get().fetchLanePaths();
    void get().fetchOverlayStatus();

    if (!isTauri()) return () => {};

    // Anchor for clock interpolation, re-set whenever GSI reports a new second.
    let anchorGameTime = useGameStore.getState().game.gameTime;
    let anchorAtMs = performance.now();
    let inFlightSince: number | null = null;
    let stopped = false;

    const tick = async () => {
      if (stopped) return;

      // Skip while a call is outstanding, but only until the deadline. A plain
      // boolean here is a one-way latch: one call that never settles would stop
      // the loop forever, leaving a stale snapshot on screen with no error.
      if (
        inFlightSince !== null &&
        performance.now() - inFlightSince < IN_FLIGHT_TIMEOUT_MS
      ) {
        return;
      }

      const { gameTime, connected } = useGameStore.getState().game;
      if (gameTime !== anchorGameTime) {
        anchorGameTime = gameTime;
        anchorAtMs = performance.now();
      }

      if (!connected) {
        if (get().snapshot !== null) set({ snapshot: null, error: null });
        return;
      }

      inFlightSince = performance.now();
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const snapshot = await invoke<WaveSnapshot>("get_wave_snapshot", {
          clockTimeSeconds: interpolatedClock(
            anchorGameTime,
            anchorAtMs,
            performance.now(),
          ),
        });
        if (!stopped) set({ snapshot, error: null });
      } catch (e) {
        console.error("Failed to fetch wave snapshot:", e);
        // Surfaced in the UI: a silently frozen map is indistinguishable from
        // one that is empty for a legitimate reason.
        if (!stopped) set({ error: String(e) });
      } finally {
        inFlightSince = null;
      }
    };

    void tick();
    const interval = setInterval(() => void tick(), TICK_INTERVAL_MS);

    return () => {
      stopped = true;
      clearInterval(interval);
    };
  },
}));
