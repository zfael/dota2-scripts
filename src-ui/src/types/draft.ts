/**
 * Live draft identification, mirroring `DraftStatusDto` in
 * `src-tauri/src/ipc_types.rs`.
 */

export interface DraftSlot {
  /** 0-9 in strip order: left team first, each side as drawn on screen. */
  index: number;
  isAlly: boolean;
  /**
   * The settled hero slug (e.g. "skeleton_king"), present only when the read
   * is trustworthy. `null` with `unknown: true` means the slot is occupied by
   * a portrait the matcher cannot identify — typically someone's arcana —
   * shown as "?" rather than a guess.
   */
  hero: string | null;
  unknown: boolean;
  agreement: number;
  bestScore: number;
}

export interface DraftStatus {
  enabled: boolean;
  /** True while the draft gate is open (a draft is on screen right now). */
  active: boolean;
  gameState: string;
  /**
   * Identity of the current draft, changing on every new one. Per-draft UI
   * state keys on this rather than `matchid`, which bot matches always report
   * as "0" — so verdicts from the previous game used to persist forever.
   */
  sessionId: string;
  matchid: string;
  teamName: string;
  ownHero: string;
  frames: number;
  slots: DraftSlot[];
}

export const EMPTY_DRAFT_STATUS: DraftStatus = {
  enabled: false,
  active: false,
  gameState: "",
  sessionId: "",
  matchid: "",
  teamName: "",
  ownHero: "",
  frames: 0,
  slots: [],
};
