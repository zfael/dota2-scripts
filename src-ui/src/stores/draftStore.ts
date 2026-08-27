import { create } from "zustand";
import type { DraftStatus } from "../types/draft";
import { EMPTY_DRAFT_STATUS } from "../types/draft";
import { isTauri } from "../lib/tauri";

interface DraftStore {
  status: DraftStatus;
  /** Slot indices whose feedback was already submitted this session. */
  judged: Record<number, "correct" | "wrong">;
  fetchStatus: () => Promise<void>;
  startPolling: () => () => void;
  submitFeedback: (
    slotIndex: number,
    correct: boolean,
    actualHero?: string,
  ) => Promise<void>;
}

// 1s matches the reader's capture cadence — polling faster only re-reads the
// same snapshot.
const POLL_INTERVAL_MS = 1000;

export const useDraftStore = create<DraftStore>((set, get) => ({
  status: EMPTY_DRAFT_STATUS,
  judged: {},

  fetchStatus: async () => {
    if (!isTauri()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const status = await invoke<DraftStatus>("get_draft_status");
      const prev = get().status;
      // A new draft invalidates the previous one's verdicts. Keyed on
      // sessionId, not matchid: bot matches report matchid as "0" every game,
      // so the verdicts stuck on "confirmed" and the buttons never came back.
      if (status.sessionId !== prev.sessionId) {
        set({ status, judged: {} });
      } else {
        set({ status });
      }
    } catch (e) {
      console.error("Failed to fetch draft status:", e);
    }
  },

  startPolling: () => {
    get().fetchStatus();
    const interval = setInterval(() => {
      get().fetchStatus();
    }, POLL_INTERVAL_MS);
    return () => clearInterval(interval);
  },

  submitFeedback: async (slotIndex, correct, actualHero) => {
    if (!isTauri()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("submit_draft_feedback", {
        slotIndex,
        correct,
        actualHero: actualHero || null,
      });
      set((state) => ({
        judged: { ...state.judged, [slotIndex]: correct ? "correct" : "wrong" },
      }));
    } catch (e) {
      console.error("Failed to submit draft feedback:", e);
    }
  },
}));
