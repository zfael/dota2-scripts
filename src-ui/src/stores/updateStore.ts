import { create } from "zustand";
import type { UpdateCheckState } from "../types/game";
import { isTauri } from "../lib/tauri";

const STARTUP_UPDATE_REFRESH_INTERVAL_MS = 750;
const STARTUP_UPDATE_REFRESH_ATTEMPTS = 20;

interface UpdateStore {
  updateState: UpdateCheckState;
  setUpdateState: (state: UpdateCheckState) => void;
  checkForUpdates: () => Promise<void>;
  applyUpdate: () => Promise<void>;
  dismissUpdate: () => void;
  loadInitialState: () => Promise<void>;
}

function shouldRefreshStartupUpdateState(state: UpdateCheckState): boolean {
  return state.kind === "idle" || state.kind === "checking";
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function fetchUpdateState(): Promise<UpdateCheckState> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<UpdateCheckState>("get_update_state");
}

export const useUpdateStore = create<UpdateStore>((set) => ({
  updateState: { kind: "idle" },

  setUpdateState: (updateState) => set({ updateState }),

  loadInitialState: async () => {
    if (!isTauri()) return;
    try {
      let state = await fetchUpdateState();
      set({ updateState: state });

      for (
        let attempt = 0;
        attempt < STARTUP_UPDATE_REFRESH_ATTEMPTS &&
        shouldRefreshStartupUpdateState(state);
        attempt += 1
      ) {
        await wait(STARTUP_UPDATE_REFRESH_INTERVAL_MS);
        state = await fetchUpdateState();
        set({ updateState: state });
      }
    } catch (e) {
      console.error("Failed to load update state:", e);
    }
  },

  checkForUpdates: async () => {
    if (!isTauri()) return;
    set({ updateState: { kind: "checking" } });
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke<UpdateCheckState>("check_for_updates");
      set({ updateState: result });
    } catch (e) {
      set({
        updateState: {
          kind: "error",
          message: e instanceof Error ? e.message : String(e),
        },
      });
    }
  },

  applyUpdate: async () => {
    if (!isTauri()) return;
    set({ updateState: { kind: "downloading" } });
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("apply_update");
    } catch (e) {
      set({
        updateState: {
          kind: "error",
          message: e instanceof Error ? e.message : String(e),
        },
      });
    }
  },

  dismissUpdate: () => {
    set({ updateState: { kind: "idle" } });
    if (isTauri()) {
      import("@tauri-apps/api/core").then(({ invoke }) => {
        invoke("dismiss_update").catch(console.error);
      });
    }
  },
}));
