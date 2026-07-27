export type Lane = "Top" | "Mid" | "Bottom";

export type Team = "Radiant" | "Dire";

/**
 * How much the prediction should be trusted. Decays with game time — the model
 * assumes undisrupted waves, which stops holding once lanes push.
 */
export type WaveConfidence = "High" | "Degrading" | "Low";

/**
 * A point in normalised map space, origin at bottom-left (Radiant corner).
 * Renderers flip the y-axis for SVG, whose origin is top-left.
 */
export interface MapPoint {
  x: number;
  y: number;
}

export interface LanePath {
  lane: Lane;
  points: MapPoint[];
}

export interface WavePosition {
  lane: Lane;
  team: Team;
  /** 0.0 = Radiant barracks end, 1.0 = Dire barracks end, for both teams. */
  progress: number;
  point: MapPoint;
  hasClashed: boolean;
}

export interface LaneClash {
  lane: Lane;
  progress: number;
  point: MapPoint;
  secondsUntilClash: number;
}

export interface WaveSnapshot {
  enabled: boolean;
  clockTimeSeconds: number;
  nextSpawnTimeSeconds: number;
  secondsUntilNextSpawn: number;
  currentWaveAgeSeconds: number | null;
  confidence: WaveConfidence;
  waves: WavePosition[];
  clashes: LaneClash[];
}

export interface OverlayBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * `dotaWindowMode` is best-effort. True exclusive fullscreen cannot be reliably
 * told apart from borderless from outside the game process, so "Borderless" means
 * "borderless or fullscreen" and the UI must say so rather than promise detection.
 */
export type DotaWindowMode = "NotFound" | "Windowed" | "Borderless";

export interface WaveOverlayStatus {
  enabled: boolean;
  visible: boolean;
  toggleKey: string;
  dotaWindowMode: DotaWindowMode;
  bounds: OverlayBounds | null;
}

export const LANE_DISPLAY_NAMES: Record<Lane, string> = {
  Top: "Top",
  Mid: "Mid",
  Bottom: "Bottom",
};

/**
 * Lane role from the perspective of each team. Bottom is the Radiant safelane and
 * the Dire offlane; Top mirrors it.
 */
export const LANE_ROLES: Record<Lane, { radiant: string; dire: string }> = {
  Top: { radiant: "Offlane", dire: "Safelane" },
  Mid: { radiant: "Mid", dire: "Mid" },
  Bottom: { radiant: "Safelane", dire: "Offlane" },
};

export const CONFIDENCE_LABELS: Record<WaveConfidence, string> = {
  High: "High confidence",
  Degrading: "Degrading — lanes may have pushed",
  Low: "Low — estimate only",
};

/** Format a game-clock second count as mm:ss. Negative clocks are pre-horn. */
export function formatGameClock(seconds: number): string {
  const negative = seconds < 0;
  const total = Math.abs(Math.floor(seconds));
  const minutes = Math.floor(total / 60);
  const remainder = total % 60;
  return `${negative ? "-" : ""}${minutes}:${String(remainder).padStart(2, "0")}`;
}
