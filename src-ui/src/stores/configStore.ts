import { create } from "zustand";
import type { Settings } from "../types/config";
import { mockConfig } from "./mockData";

interface ConfigStore {
  config: Settings;
  updateConfig: <K extends keyof Settings>(
    section: K,
    updates: Partial<Settings[K]>,
  ) => void;
  updateHeroConfig: <K extends keyof Settings["heroes"]>(
    hero: K,
    updates: Partial<Settings["heroes"][K]>,
  ) => void;
}

export const useConfigStore = create<ConfigStore>((set) => ({
  config: mockConfig,

  updateConfig: (section, updates) =>
    set((state) => ({
      config: {
        ...state.config,
        [section]: { ...state.config[section], ...updates },
      },
    })),

  updateHeroConfig: (hero, updates) =>
    set((state) => ({
      config: {
        ...state.config,
        heroes: {
          ...state.config.heroes,
          [hero]: { ...state.config.heroes[hero], ...updates },
        },
      },
    })),
}));
