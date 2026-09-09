import { create } from "zustand";
import type { Settings } from "../types/config";
import { mockConfig } from "./mockData";
import { isTauri } from "../lib/tauri";

/**
 * Broadcast by Rust after a config write, carrying the full persisted `Settings`.
 *
 * Keeps the overlay — a separate webview with its own store — from rendering
 * settings it loaded once, the first time it was opened, and never revisited.
 */
export const CONFIG_UPDATED_EVENT = "config_updated";

interface ConfigStore {
  config: Settings;
  loaded: boolean;
  /**
   * The last write that Rust rejected, or null.
   *
   * A rejected write used to be a bare `console.error`, so the UI kept showing
   * the value it had optimistically applied and the user only found out the
   * setting had not saved when they restarted the app (issue #20).
   */
  saveError: string | null;
  clearSaveError: () => void;
  loadConfig: () => Promise<void>;
  startListening: () => Promise<() => void>;
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

/**
 * Sections this window has edited but not yet finished persisting.
 *
 * The broadcast goes to every window including this one, and it carries the whole
 * `Settings`. Applying it while another section's write is still queued would roll
 * that section back to its pre-edit value until its own timer fires. Local state is
 * already correct for anything we wrote, so ignoring the echo costs nothing.
 */
const pendingWrites = new Set<string>();

/** Exported for tests; there is no way to observe this from outside otherwise. */
export function hasPendingConfigWrites(): boolean {
  return pendingWrites.size > 0;
}

function debouncedInvoke(
  key: string,
  command: string,
  args: Record<string, unknown>,
  onError: (message: string) => void,
) {
  if (!isTauri()) return;

  if (debounceTimers[key]) clearTimeout(debounceTimers[key]);
  pendingWrites.add(key);

  const handle = setTimeout(async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke(command, args);
    } catch (e) {
      console.error(`Failed to persist '${key}':`, e);
      onError(e instanceof Error ? e.message : String(e));
    } finally {
      // Only stand down if no newer edit to this section was queued while the
      // call was in flight — otherwise that edit's echo would slip through.
      // Cleared even on failure: a stuck entry would block every later broadcast.
      if (debounceTimers[key] === handle) {
        pendingWrites.delete(key);
        delete debounceTimers[key];
      }
    }
  }, DEBOUNCE_MS);
  debounceTimers[key] = handle;
}

function debouncedPersist(
  section: string,
  updates: Record<string, unknown>,
  onError: (message: string) => void,
) {
  debouncedInvoke(`config:${section}`, "update_config", { section, updates }, onError);
}

function debouncedPersistHero(
  hero: string,
  updates: Record<string, unknown>,
  onError: (message: string) => void,
) {
  debouncedInvoke(`hero:${hero}`, "update_hero_config", { hero, updates }, onError);
}

export const useConfigStore = create<ConfigStore>((set) => ({
  config: mockConfig,
  loaded: false,
  saveError: null,
  clearSaveError: () => set({ saveError: null }),

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

  startListening: async () => {
    if (!isTauri()) return () => {};

    const { listen } = await import("@tauri-apps/api/event");
    return listen<Settings>(CONFIG_UPDATED_EVENT, (event) => {
      if (hasPendingConfigWrites()) return;
      set({ config: event.payload, loaded: true });
    });
  },

  updateConfig: (section, updates) => {
    set((state) => {
      // Kept so a rejected write can put back exactly what was on screen
      // before, rather than leaving a value the backend refused to store.
      const previous = state.config[section];
      const newConfig = {
        ...state.config,
        [section]: { ...state.config[section], ...updates },
      };
      debouncedPersist(section, updates as Record<string, unknown>, (message) =>
        set({ config: { ...newConfig, [section]: previous }, saveError: message }),
      );
      return { config: newConfig, saveError: null };
    });
  },

  updateHeroConfig: (hero, updates) => {
    set((state) => {
      const previous = state.config.heroes[hero];
      const newConfig = {
        ...state.config,
        heroes: {
          ...state.config.heroes,
          [hero]: { ...state.config.heroes[hero], ...updates },
        },
      };
      debouncedPersistHero(hero, updates as Record<string, unknown>, (message) =>
        set({
          config: { ...newConfig, heroes: { ...newConfig.heroes, [hero]: previous } },
          saveError: message,
        }),
      );
      return { config: newConfig, saveError: null };
    });
  },
}));
