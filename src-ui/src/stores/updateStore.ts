import { create } from "zustand";
import type { UpdateCheckState } from "../types/game";
import { isTauri } from "../lib/tauri";

interface UpdateStore {
  updateState: UpdateCheckState;
  setUpdateState: (state: UpdateCheckState) => void;
  checkForUpdates: () => Promise<void>;
  applyUpdate: () => Promise<void>;
  dismissUpdate: () => void;
  loadInitialState: () => Promise<void>;
}

export const useUpdateStore = create<UpdateStore>((set) => ({
  updateState: { kind: "idle" },

  setUpdateState: (updateState) => set({ updateState }),

  loadInitialState: async () => {
    if (!isTauri()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const state = await invoke<UpdateCheckState>("get_update_state");
      set({ updateState: state });
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
