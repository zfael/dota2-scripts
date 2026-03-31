import { create } from "zustand";
import type { GameState, DiagnosticsState } from "../types/game";
import { isTauri } from "../lib/tauri";

interface GameStore {
  game: GameState;
  diagnostics: DiagnosticsState;
  setGame: (game: Partial<GameState>) => void;
  setDiagnostics: (diagnostics: DiagnosticsState) => void;
  startListening: () => Promise<void>;
}

export const useGameStore = create<GameStore>((set) => ({
  game: {
    heroName: null,
    heroLevel: 0,
    hpPercent: 100,
    manaPercent: 100,
    inDanger: false,
    connected: false,
    alive: true,
    stunned: false,
    silenced: false,
    respawnTimer: null,
    runeTimer: null,
    gameTime: 0,
  },
  diagnostics: {
    gsiConnected: false,
    keyboardHookActive: false,
    queueMetrics: {
      eventsProcessed: 0,
      eventsDropped: 0,
      currentQueueDepth: 0,
      maxQueueDepth: 10,
    },
    syntheticInput: {
      queueDepth: 0,
      totalQueued: 0,
      peakDepth: 0,
      completions: 0,
      drops: 0,
    },
    soulRingState: "ready",
    blockedKeys: [],
  },

  setGame: (partial) =>
    set((state) => ({ game: { ...state.game, ...partial } })),

  setDiagnostics: (diagnostics) => set({ diagnostics }),

  startListening: async () => {
    if (!isTauri()) return;

    const { listen } = await import("@tauri-apps/api/event");

    // Subscribe to real-time game state updates from Rust
    listen<GameState>("gsi_update", (event) => {
      set({ game: event.payload });
    });

    // Initial diagnostics fetch
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const diag = await invoke<DiagnosticsState>("get_diagnostics");
      set({ diagnostics: diag });
    } catch (e) {
      console.error("Failed to fetch diagnostics:", e);
    }
  },
}));
