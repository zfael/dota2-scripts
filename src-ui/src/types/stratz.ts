/**
 * STRATZ-backed draft advice, mirroring `StratzStatusDto` and
 * `DraftAdviceDto` in `src-tauri/src/ipc_types.rs`.
 *
 * Note there is no token field anywhere here, by design: the backend reports
 * only whether one is set. The value never crosses into the webview.
 */

export interface StratzStatus {
  enabled: boolean;
  hasToken: boolean;
  /** Position 1-5 being queued for; 0 means no role filter. */
  position: number;
  /** Suggestions are restricted to commonly picked heroes. */
  metaOnly: boolean;
  /** A usable dataset is loaded. */
  ready: boolean;
  refreshing: boolean;
  /** 0-100 while refreshing. */
  progress: number;
  heroCount: number;
  /**
   * Heroes whose matchups the refresh could not fetch — STRATZ failing
   * requests during the build. They still appear as suggestions but carry no
   * counter or synergy signal, so this is shown rather than hidden.
   */
  incompleteHeroes: number;
  bracket: string;
  /** Unix seconds the dataset was built. */
  builtAt: number;
  lastError: string | null;
}

/** How a suggestion relates to one hero already in the draft. */
export interface MatchupDetail {
  slug: string;
  displayName: string;
  /**
   * Win-rate offset over this pick's own baseline, as a fraction: +0.05 means
   * five points better than the hero's average.
   */
  offset: number;
  matches: number;
  /** The sample-shrunk value that actually entered the score. */
  contribution: number;
}

export interface Suggestion {
  slug: string;
  displayName: string;
  score: number;
  /** How much of the score comes from countering the enemy lineup. */
  counter: number;
  /** How much comes from working with our own picks. */
  synergy: number;
  /** Win rate in the selected role, where measured. */
  positionWinRate: number | null;
  /**
   * Share of matches this hero is picked in — restricted to the selected role
   * when there is one. `null` where the refresh never fetched the hero.
   */
  pickRate: number | null;
  /** The enemy this pick most counters. */
  bestAgainst: string | null;
  /** Every enemy in draft order, so weak matchups are visible too. */
  vsEnemies: MatchupDetail[];
  withAllies: MatchupDetail[];
  /** Games behind the counter term, so the UI can show its weight. */
  counterSamples: number;
}

export interface DraftAdvice {
  suggestions: Suggestion[];
  /**
   * Identified heroes the dataset did not recognise — a cache older than the
   * current patch. Shown so the advice is not presented as complete when it
   * is missing a pick.
   */
  unresolved: string[];
  alliesUsed: number;
  enemiesUsed: number;
}

export const EMPTY_STRATZ_STATUS: StratzStatus = {
  enabled: false,
  hasToken: false,
  position: 0,
  metaOnly: false,
  ready: false,
  refreshing: false,
  progress: 0,
  heroCount: 0,
  incompleteHeroes: 0,
  bracket: "",
  builtAt: 0,
  lastError: null,
};

export const EMPTY_DRAFT_ADVICE: DraftAdvice = {
  suggestions: [],
  unresolved: [],
  alliesUsed: 0,
  enemiesUsed: 0,
};

/** Role names on their own, for inline prose like "54% win as Mid". */
export const POSITION_SHORT: Record<number, string> = {
  1: "Carry",
  2: "Mid",
  3: "Offlane",
  4: "Soft support",
  5: "Hard support",
};

export const POSITION_LABELS: Record<number, string> = {
  1: "Pos 1 · Carry",
  2: "Pos 2 · Mid",
  3: "Pos 3 · Offlane",
  4: "Pos 4 · Soft support",
  5: "Pos 5 · Hard support",
};
