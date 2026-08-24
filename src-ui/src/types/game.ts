export type HeroType =
  | "broodmother"
  | "earth_spirit"
  | "ember_spirit"
  | "huskar"
  | "invoker"
  | "largo"
  | "legion_commander"
  | "magnus"
  | "meepo"
  | "mirana"
  | "outworld_destroyer"
  | "shadow_fiend"
  | "slark"
  | "snapfire"
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
  { id: "earth_spirit", displayName: "Earth Spirit", internalName: "npc_dota_hero_earth_spirit", icon: "🗿", role: "Support / Disabler" },
  { id: "ember_spirit", displayName: "Ember Spirit", internalName: "npc_dota_hero_ember_spirit", icon: "🌋", role: "Carry / Escape" },
  { id: "huskar", displayName: "Huskar", internalName: "npc_dota_hero_huskar", icon: "🔥", role: "Carry / Durable" },
  { id: "invoker", displayName: "Invoker", internalName: "npc_dota_hero_invoker", icon: "🔮", role: "Carry / Nuker" },
  { id: "largo", displayName: "Largo", internalName: "npc_dota_hero_largo", icon: "🎵", role: "Support / Healer" },
  { id: "legion_commander", displayName: "Legion Commander", internalName: "npc_dota_hero_legion_commander", icon: "⚔️", role: "Initiator / Durable" },
  { id: "magnus", displayName: "Magnus", internalName: "npc_dota_hero_magnataur", icon: "🦏", role: "Initiator / Disabler" },
  { id: "meepo", displayName: "Meepo", internalName: "npc_dota_hero_meepo", icon: "🐾", role: "Carry / Escape" },
  { id: "mirana", displayName: "Mirana", internalName: "npc_dota_hero_mirana", icon: "🌙", role: "Carry / Escape" },
  { id: "outworld_destroyer", displayName: "Outworld Destroyer", internalName: "npc_dota_hero_obsidian_destroyer", icon: "🌀", role: "Carry / Nuker" },
  { id: "shadow_fiend", displayName: "Shadow Fiend", internalName: "npc_dota_hero_nevermore", icon: "👻", role: "Carry / Nuker" },
  { id: "slark", displayName: "Slark", internalName: "npc_dota_hero_slark", icon: "🐟", role: "Carry / Escape" },
  { id: "snapfire", displayName: "Snapfire", internalName: "npc_dota_hero_snapfire", icon: "🍪", role: "Support / Nuker" },
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
  /** Payloads Dota sent that the Rust schema could not parse. */
  eventsRejected: number;
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

export interface MeepoObservedState {
  healthPercent: number;
  manaPercent: number;
  inDanger: boolean;
  alive: boolean;
  stunned: boolean;
  silenced: boolean;
  poofReady: boolean;
  digReady: boolean;
  megameepoReady: boolean;
  hasShard: boolean;
  hasScepter: boolean;
  blinkAvailable: boolean;
  comboItems: string[];
}
