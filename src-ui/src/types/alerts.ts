/** Stable event keys, matching `AlertEvent::key()` in Rust. */
export type AlertEventKey =
  | "power_rune"
  | "wisdom_rune"
  | "water_rune"
  | "bounty_rune"
  | "tormentor"
  | "neutral_item"
  | "stack";

export interface AlertCountdown {
  event: AlertEventKey;
  displayName: string;
  enabled: boolean;
  /** Null when a fixed schedule has run out — water runes after 4:00, say. */
  nextOccurrenceSeconds: number | null;
  secondsUntil: number | null;
}

/**
 * Display order and the cue description for each event.
 *
 * The cue text is shown in the UI so the mapping from sound to event can be
 * learned by reading rather than by trial and error.
 */
export const ALERT_EVENTS: {
  key: AlertEventKey;
  label: string;
  schedule: string;
  cue: string;
}[] = [
  {
    key: "power_rune",
    label: "Power Rune",
    schedule: "Every 2 min from 6:00",
    cue: "Two rising bell blips",
  },
  {
    key: "wisdom_rune",
    label: "Wisdom Rune",
    schedule: "Every 7 min from 7:00",
    cue: "Three rising wooden notes",
  },
  {
    key: "water_rune",
    label: "Water Rune",
    schedule: "2:00 and 4:00 only",
    cue: "One soft drop",
  },
  {
    key: "bounty_rune",
    label: "Bounty Rune",
    schedule: "Every 3 min from 0:00",
    cue: "Two quick high ticks",
  },
  {
    key: "tormentor",
    label: "Tormentor",
    schedule: "20:00, then every 10 min",
    cue: "Falling brass",
  },
  {
    key: "neutral_item",
    label: "Neutral Item",
    schedule: "7 / 17 / 27 / 37 / 60 min",
    cue: "Four rising plucked notes",
  },
  {
    key: "stack",
    label: "Stack Timing",
    schedule: "Every minute at :53",
    cue: "One dry tick",
  },
];

/** Format a countdown as mm:ss, or a dash when nothing is scheduled. */
export function formatCountdown(seconds: number | null): string {
  if (seconds === null) return "—";
  if (seconds < 0) return "now";
  const minutes = Math.floor(seconds / 60);
  const remainder = seconds % 60;
  return `${minutes}:${String(remainder).padStart(2, "0")}`;
}
