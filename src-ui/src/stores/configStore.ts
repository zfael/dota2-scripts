import { create } from "zustand";
import type { Settings } from "../types/config";
import { mockConfig } from "./mockData";
import { isTauri } from "../lib/tauri";

interface ConfigStore {
  config: Settings;
  loaded: boolean;
  loadConfig: () => Promise<void>;
  updateConfig: <K extends keyof Settings>(
    section: K,
    updates: Partial<Settings[K]>,
  ) => void;
  updateHeroConfig: <K extends keyof Settings["heroes"]>(
    hero: K,
    updates: Partial<Settings["heroes"][K]>,
  ) => void;
}

// Debounce timers per section
const debounceTimers: Record<string, ReturnType<typeof setTimeout>> = {};
const DEBOUNCE_MS = 300;

function debouncedPersist(section: string, updates: Record<string, unknown>) {
  if (!isTauri()) return;

  const key = `config:${section}`;
  if (debounceTimers[key]) clearTimeout(debounceTimers[key]);

  debounceTimers[key] = setTimeout(async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("update_config", { section, updates });
    } catch (e) {
      console.error(`Failed to persist config section '${section}':`, e);
    }
  }, DEBOUNCE_MS);
}

function debouncedPersistHero(hero: string, updates: Record<string, unknown>) {
  if (!isTauri()) return;

  const key = `hero:${hero}`;
  if (debounceTimers[key]) clearTimeout(debounceTimers[key]);

  debounceTimers[key] = setTimeout(async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("update_hero_config", { hero, updates });
    } catch (e) {
      console.error(`Failed to persist hero config '${hero}':`, e);
    }
  }, DEBOUNCE_MS);
}

export const useConfigStore = create<ConfigStore>((set) => ({
  config: mockConfig,
  loaded: false,

  loadConfig: async () => {
    if (!isTauri()) {
      set({ loaded: true });
      return;
    }
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const config = await invoke<Settings>("get_config");
      set({ config, loaded: true });
    } catch (e) {
      console.error("Failed to load config:", e);
      set({ loaded: true });
    }
  },

  updateConfig: (section, updates) => {
    set((state) => {
      const newConfig = {
        ...state.config,
        [section]: { ...state.config[section], ...updates },
      };
      debouncedPersist(section, updates as Record<string, unknown>);
      return { config: newConfig };
    });
  },

  updateHeroConfig: (hero, updates) => {
    set((state) => {
      const newConfig = {
        ...state.config,
        heroes: {
          ...state.config.heroes,
          [hero]: { ...state.config.heroes[hero], ...updates },
        },
      };
      debouncedPersistHero(hero, updates as Record<string, unknown>);
      return { config: newConfig };
    });
  },
}));
