import { create } from "zustand";
import type { ActivityEntry, ActivityCategory } from "../types/activity";
import { isTauri } from "../lib/tauri";

interface ActivityStore {
  entries: ActivityEntry[];
  filter: ActivityCategory | "all";
  setFilter: (filter: ActivityCategory | "all") => void;
  addEntry: (entry: ActivityEntry) => void;
  clear: () => void;
  filteredEntries: () => ActivityEntry[];
  startListening: () => Promise<() => void>;
}

export const useActivityStore = create<ActivityStore>((set, get) => ({
  entries: [],
  filter: "all",
  setFilter: (filter) => set({ filter }),
  addEntry: (entry) =>
    set((state) => ({ entries: [...state.entries.slice(-499), entry] })),
  clear: () => set({ entries: [] }),
  filteredEntries: () => {
    const { entries, filter } = get();
    return filter === "all"
      ? entries
      : entries.filter((e) => e.category === filter);
  },
  startListening: async () => {
    if (!isTauri()) return () => {};

    const { listen } = await import("@tauri-apps/api/event");

    const unlisten = await listen<ActivityEntry>("activity_event", (event) => {
      get().addEntry(event.payload);
    });

    return unlisten;
  },
}));
