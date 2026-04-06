import { create } from "zustand";
import { isTauri } from "../lib/tauri";

interface UIStore {
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
  gsiEnabled: boolean;
  standaloneEnabled: boolean;
  appVersion: string;
  armletRoshanArmed: boolean;
  setGsiEnabled: (enabled: boolean) => void;
  setStandaloneEnabled: (enabled: boolean) => void;
  setArmletRoshanArmed: (armed: boolean) => void;
  loadInitialState: () => Promise<void>;
  startListening: () => Promise<() => void>;
}

export const useUIStore = create<UIStore>((set) => ({
  sidebarCollapsed: false,
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  gsiEnabled: true,
  standaloneEnabled: false,
  appVersion: "0.1.0",
  armletRoshanArmed: false,

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

  setArmletRoshanArmed: (armed) => {
    set({ armletRoshanArmed: armed });
    if (isTauri()) {
      import("@tauri-apps/api/core").then(({ invoke }) => {
        invoke("set_armlet_roshan_mode_armed", { armed }).catch(console.error);
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
        armletRoshanArmed: boolean;
        appVersion: string;
      }>("get_app_state");
      set({
        gsiEnabled: state.gsiEnabled,
        standaloneEnabled: state.standaloneEnabled,
        armletRoshanArmed: state.armletRoshanArmed,
        appVersion: state.appVersion,
      });
    } catch (e) {
      console.error("Failed to load app state:", e);
    }
  },

  startListening: async () => {
    if (!isTauri()) return () => {};

    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<{
      selectedHero: string | null;
      gsiEnabled: boolean;
      standaloneEnabled: boolean;
      armletRoshanArmed: boolean;
      appVersion: string;
    }>("app_state_update", (event) => {
      set({
        gsiEnabled: event.payload.gsiEnabled,
        standaloneEnabled: event.payload.standaloneEnabled,
        armletRoshanArmed: event.payload.armletRoshanArmed,
        appVersion: event.payload.appVersion,
      });
    });

    return unlisten;
  },
}));
