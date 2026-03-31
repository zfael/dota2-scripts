import { create } from "zustand";
import type { GameState, DiagnosticsState, UpdateCheckState } from "../types/game";

interface GameStore {
  game: GameState;
  diagnostics: DiagnosticsState;
  updateState: UpdateCheckState;
  setGame: (game: Partial<GameState>) => void;
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
    queueMetrics: { eventsProcessed: 0, eventsDropped: 0, currentQueueDepth: 0, maxQueueDepth: 10 },
    syntheticInput: { queueDepth: 0, totalQueued: 0, peakDepth: 0, completions: 0, drops: 0 },
    soulRingState: "ready",
    blockedKeys: [],
  },
  updateState: { kind: "idle" },
  setGame: (partial) =>
    set((state) => ({ game: { ...state.game, ...partial } })),
}));
