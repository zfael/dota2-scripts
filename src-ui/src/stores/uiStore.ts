import { create } from "zustand";
import { isTauri } from "../lib/tauri";

interface UIStore {
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
  gsiEnabled: boolean;
  standaloneEnabled: boolean;
  appVersion: string;
  setGsiEnabled: (enabled: boolean) => void;
  setStandaloneEnabled: (enabled: boolean) => void;
  loadInitialState: () => Promise<void>;
}

export const useUIStore = create<UIStore>((set) => ({
  sidebarCollapsed: false,
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  gsiEnabled: true,
  standaloneEnabled: false,
  appVersion: "0.1.0",

  setGsiEnabled: (enabled) => {
    set({ gsiEnabled: enabled });
    if (isTauri()) {
      import("@tauri-apps/api/core").then(({ invoke }) => {
        invoke("set_gsi_enabled", { enabled }).catch(console.error);
      });
    }
  },

  setStandaloneEnabled: (enabled) => {
    set({ standaloneEnabled: enabled });
    if (isTauri()) {
      import("@tauri-apps/api/core").then(({ invoke }) => {
        invoke("set_standalone_enabled", { enabled }).catch(console.error);
      });
    }
  },

  loadInitialState: async () => {
    if (!isTauri()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const state = await invoke<{
        selectedHero: string | null;
        gsiEnabled: boolean;
        standaloneEnabled: boolean;
        appVersion: string;
      }>("get_app_state");
      set({
        gsiEnabled: state.gsiEnabled,
        standaloneEnabled: state.standaloneEnabled,
        appVersion: state.appVersion,
      });
    } catch (e) {
      console.error("Failed to load app state:", e);
    }
  },
}));
