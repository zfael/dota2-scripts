export type MapZone =
  | "TopLane"
  | "MidLane"
  | "BotLane"
  | "DireJungle"
  | "RadiantJungle"
  | "Roshan"
  | "Other";

export type ActivityLevel = "Quiet" | "Active" | "Fight";

export interface ZoneSummary {
  zone: MapZone;
  avgAllyCount: number;
  avgEnemyCount: number;
  peakActivity: ActivityLevel;
  currentActivity: ActivityLevel;
  framesWithFight: number;
}

export type LaneEventType = "FightDetected" | "FightOngoing" | "EnemyRotation" | "EnemyGrouping";

export interface LaneEvent {
  id: string;
  timestamp: string;
  type: LaneEventType;
  zone: MapZone;
  count?: number;
}

export interface MinimapStatus {
  enabled: boolean;
  health: "idle" | "healthy" | "unhealthy";
  captureIntervalMs: number;
  windowBindingStatus: string;
  consecutiveFailures: number;
  lastCaptureDurationMs: number | null;
  samplingMode: string;
}

export const ZONE_DISPLAY_NAMES: Record<MapZone, string> = {
  TopLane: "Top Lane",
  MidLane: "Mid Lane",
  BotLane: "Bot Lane",
  DireJungle: "Dire Jungle",
  RadiantJungle: "Radiant Jungle",
  Roshan: "Roshan",
  Other: "Other",
};

export const ZONE_ICONS: Record<MapZone, string> = {
  TopLane: "⬆",
  MidLane: "↔",
  BotLane: "⬇",
  DireJungle: "🌲",
  RadiantJungle: "🌿",
  Roshan: "💀",
  Other: "◌",
};
