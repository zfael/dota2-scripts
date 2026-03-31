export type HeroType =
  | "broodmother"
  | "huskar"
  | "largo"
  | "legion_commander"
  | "meepo"
  | "outworld_destroyer"
  | "shadow_fiend"
  | "tiny";

export interface HeroInfo {
  id: HeroType;
  displayName: string;
  internalName: string;
  icon: string;
  role: string;
}

export const HEROES: HeroInfo[] = [
  { id: "broodmother", displayName: "Broodmother", internalName: "npc_dota_hero_broodmother", icon: "🕷️", role: "Pusher / Carry" },
  { id: "huskar", displayName: "Huskar", internalName: "npc_dota_hero_huskar", icon: "🔥", role: "Carry / Durable" },
  { id: "largo", displayName: "Largo", internalName: "npc_dota_hero_largo", icon: "🎵", role: "Support / Healer" },
  { id: "legion_commander", displayName: "Legion Commander", internalName: "npc_dota_hero_legion_commander", icon: "⚔️", role: "Initiator / Durable" },
  { id: "meepo", displayName: "Meepo", internalName: "npc_dota_hero_meepo", icon: "🐾", role: "Carry / Escape" },
  { id: "outworld_destroyer", displayName: "Outworld Destroyer", internalName: "npc_dota_hero_obsidian_destroyer", icon: "🌀", role: "Carry / Nuker" },
  { id: "shadow_fiend", displayName: "Shadow Fiend", internalName: "npc_dota_hero_nevermore", icon: "👻", role: "Carry / Nuker" },
  { id: "tiny", displayName: "Tiny", internalName: "npc_dota_hero_tiny", icon: "🪨", role: "Initiator / Nuker" },
];

export type UpdateCheckState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "available"; version: string; releaseNotes?: string }
  | { kind: "downloading" }
  | { kind: "error"; message: string }
  | { kind: "upToDate" };

export interface GameState {
  heroName: string | null;
  heroLevel: number;
  hpPercent: number;
  manaPercent: number;
  inDanger: boolean;
  connected: boolean;
  alive: boolean;
  stunned: boolean;
  silenced: boolean;
  respawnTimer: number | null;
  runeTimer: number | null;
  gameTime: number;
}

export interface QueueMetrics {
  eventsProcessed: number;
  eventsDropped: number;
  currentQueueDepth: number;
  maxQueueDepth: number;
}

export interface DiagnosticsState {
  gsiConnected: boolean;
  keyboardHookActive: boolean;
  queueMetrics: QueueMetrics;
  syntheticInput: {
    queueDepth: number;
    totalQueued: number;
    peakDepth: number;
    completions: number;
    drops: number;
  };
  soulRingState: "ready" | "triggered" | "cooldown";
  blockedKeys: string[];
}
