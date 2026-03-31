import { create } from "zustand";

interface UIStore {
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
  gsiEnabled: boolean;
  standaloneEnabled: boolean;
  setGsiEnabled: (enabled: boolean) => void;
  setStandaloneEnabled: (enabled: boolean) => void;
}

export const useUIStore = create<UIStore>((set) => ({
  sidebarCollapsed: false,
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  gsiEnabled: true,
  standaloneEnabled: false,
  setGsiEnabled: (enabled) => set({ gsiEnabled: enabled }),
  setStandaloneEnabled: (enabled) => set({ standaloneEnabled: enabled }),
}));
