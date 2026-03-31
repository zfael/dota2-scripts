import { create } from "zustand";
import type { ActivityEntry, ActivityCategory } from "../types/activity";
import { mockActivityLog } from "./mockData";

interface ActivityStore {
  entries: ActivityEntry[];
  filter: ActivityCategory | "all";
  setFilter: (filter: ActivityCategory | "all") => void;
  addEntry: (entry: ActivityEntry) => void;
  clear: () => void;
  filteredEntries: () => ActivityEntry[];
}

export const useActivityStore = create<ActivityStore>((set, get) => ({
  entries: mockActivityLog,
  filter: "all",
  setFilter: (filter) => set({ filter }),
  addEntry: (entry) =>
    set((state) => ({ entries: [...state.entries.slice(-499), entry] })),
  clear: () => set({ entries: [] }),
  filteredEntries: () => {
    const { entries, filter } = get();
    return filter === "all" ? entries : entries.filter((e) => e.category === filter);
  },
}));
