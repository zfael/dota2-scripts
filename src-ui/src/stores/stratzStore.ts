import { create } from "zustand";
import type { DraftAdvice, StratzStatus } from "../types/stratz";
import { EMPTY_DRAFT_ADVICE, EMPTY_STRATZ_STATUS } from "../types/stratz";
import { isTauri } from "../lib/tauri";

interface StratzStore {
  status: StratzStatus;
  advice: DraftAdvice;
  /** True while a token is being validated against the API. */
  savingToken: boolean;
  tokenError: string | null;
  fetchStatus: () => Promise<void>;
  startPolling: () => () => void;
  /** Recompute advice. Cheap — local matrix maths, no network. */
  fetchAdvice: () => Promise<void>;
  saveToken: (token: string) => Promise<boolean>;
  clearToken: () => Promise<void>;
  setPosition: (position: number) => Promise<void>;
  setMetaOnly: (metaOnly: boolean) => Promise<void>;
  /** Rebuild the dataset now, without waiting for the cache to expire. */
  refreshDataset: () => Promise<void>;
  /**
   * True between the click and the worker reporting `refreshing`. The worker
   * picks the request up within a second, and a button that does nothing for
   * that second reads as broken.
   */
  requestingRefresh: boolean;
  refreshError: string | null;
}

// Matches the reader's capture cadence; the dataset itself changes at most
// once a day, so this is really just for refresh progress.
const POLL_INTERVAL_MS = 1000;

export const useStratzStore = create<StratzStore>((set, get) => ({
  status: EMPTY_STRATZ_STATUS,
  advice: EMPTY_DRAFT_ADVICE,
  savingToken: false,
  tokenError: null,
  requestingRefresh: false,
  refreshError: null,

  fetchStatus: async () => {
    if (!isTauri()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const status = await invoke<StratzStatus>("get_stratz_status");
      set((s) => ({
        status,
        // The worker has taken the request; its own flag drives the UI now.
        requestingRefresh: s.requestingRefresh && !status.refreshing,
      }));
    } catch (e) {
      console.error("Failed to fetch STRATZ status:", e);
    }
  },

  startPolling: () => {
    get().fetchStatus();
    const interval = setInterval(() => get().fetchStatus(), POLL_INTERVAL_MS);
    return () => clearInterval(interval);
  },

  fetchAdvice: async () => {
    if (!isTauri()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      set({ advice: await invoke<DraftAdvice>("get_draft_advice") });
    } catch (e) {
      console.error("Failed to fetch draft advice:", e);
    }
  },

  saveToken: async (token) => {
    if (!isTauri()) return false;
    set({ savingToken: true, tokenError: null });
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      // The backend validates against the API before saving, so a typo
      // surfaces here rather than as a silent failed refresh a minute later.
      await invoke("set_stratz_token", { token });
      await get().fetchStatus();
      set({ savingToken: false });
      return true;
    } catch (e) {
      set({ savingToken: false, tokenError: String(e) });
      return false;
    }
  },

  clearToken: async () => {
    if (!isTauri()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("clear_stratz_token");
      set({ advice: EMPTY_DRAFT_ADVICE, tokenError: null });
      await get().fetchStatus();
    } catch (e) {
      console.error("Failed to clear STRATZ token:", e);
    }
  },

  setPosition: async (position) => {
    if (!isTauri()) return;
    // Optimistic: the role selector must feel instant, and the backend value
    // arrives on the next poll anyway.
    set((s) => ({ status: { ...s.status, position } }));
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("update_config", { section: "stratz", updates: { position } });
      await get().fetchAdvice();
    } catch (e) {
      console.error("Failed to set draft position:", e);
    }
  },

  setMetaOnly: async (metaOnly) => {
    if (!isTauri()) return;
    // Optimistic for the same reason as the role selector: the list has to
    // change the instant the toggle moves.
    set((s) => ({ status: { ...s.status, metaOnly } }));
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("update_config", {
        section: "stratz",
        updates: { meta_only: metaOnly },
      });
      await get().fetchAdvice();
    } catch (e) {
      console.error("Failed to set the meta filter:", e);
    }
  },

  refreshDataset: async () => {
    if (!isTauri()) return;
    set({ requestingRefresh: true, refreshError: null });
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("refresh_stratz_dataset");
      await get().fetchStatus();
    } catch (e) {
      // Refused before it started — no token, or one already running.
      set({ requestingRefresh: false, refreshError: String(e) });
    }
  },
}));
