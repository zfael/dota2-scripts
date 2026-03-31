import { create } from "zustand";
import type { MinimapStatus, ZoneSummary, LaneEvent } from "../types/minimap";
import { isTauri } from "../lib/tauri";

interface MinimapStore {
  status: MinimapStatus;
  zones: ZoneSummary[];
  events: LaneEvent[];
  loading: boolean;
  fetchStatus: () => Promise<void>;
  addEvent: (event: LaneEvent) => void;
  setZones: (zones: ZoneSummary[]) => void;
  startPolling: () => () => void;
}

const POLL_INTERVAL_MS = 2000;
const MAX_EVENTS = 50;

export const useMinimapStore = create<MinimapStore>((set, get) => ({
  status: {
    enabled: false,
    health: "idle",
    captureIntervalMs: 0,
    windowBindingStatus: "unknown",
    consecutiveFailures: 0,
    lastCaptureDurationMs: null,
    samplingMode: "disabled",
  },
  zones: [],
  events: [],
  loading: false,

  fetchStatus: async () => {
    if (!isTauri()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const status = await invoke<MinimapStatus>("get_minimap_status");
      set({ status });
    } catch (e) {
      console.error("Failed to fetch minimap status:", e);
    }
  },

  addEvent: (event) => {
    set((state) => ({
      events: [event, ...state.events].slice(0, MAX_EVENTS),
    }));
  },

  setZones: (zones) => set({ zones }),

  startPolling: () => {
    // Initial fetch
    get().fetchStatus();

    const interval = setInterval(() => {
      get().fetchStatus();
    }, POLL_INTERVAL_MS);

    return () => clearInterval(interval);
  },
}));
