import { useRef, useEffect, useState } from "react";
import { useActivityStore } from "../stores/activityStore";
import { Button } from "../components/common/Button";
import type { ActivityCategory } from "../types/activity";

const filters: { label: string; value: ActivityCategory | "all" }[] = [
  { label: "All", value: "all" },
  { label: "Actions", value: "action" },
  { label: "Danger", value: "danger" },
  { label: "Errors", value: "error" },
  { label: "System", value: "system" },
];

const categoryColors: Record<string, string> = {
  action: "text-terminal",
  danger: "text-danger",
  warning: "text-warning",
  system: "text-info",
  error: "text-danger",
};

export default function ActivityLog() {
  const entries = useActivityStore((s) => s.filteredEntries());
  const filter = useActivityStore((s) => s.filter);
  const setFilter = useActivityStore((s) => s.setFilter);
  const clear = useActivityStore((s) => s.clear);
  const [paused, setPaused] = useState(false);
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!paused) {
      endRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [entries.length, paused]);

  return (
    <div className="flex h-full flex-col p-6">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-xl font-semibold">Activity Log</h2>
        <div className="flex items-center gap-2">
          <Button
            variant="secondary"
            onClick={() => setPaused(!paused)}
          >
            {paused ? "Resume" : "Pause"}
          </Button>
          <Button variant="danger" onClick={clear}>
            Clear
          </Button>
        </div>
      </div>

      <div className="mb-4 flex gap-2">
        {filters.map((f) => (
          <button
            key={f.value}
            type="button"
            onClick={() => setFilter(f.value)}
            className={`rounded-full px-3 py-1 text-xs font-medium transition-colors ${
              filter === f.value
                ? "bg-gold text-base"
                : "bg-elevated text-subtle hover:text-content"
            }`}
          >
            {f.label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto rounded-lg bg-terminal-bg p-4 font-mono text-xs">
        {entries.length === 0 ? (
          <p className="text-muted">No activity entries.</p>
        ) : (
          <div className="space-y-0.5">
            {entries.map((entry) => (
              <div key={entry.id} className="flex gap-3">
                <span className="shrink-0 text-muted">&gt; {entry.timestamp}</span>
                <span className={`shrink-0 w-16 uppercase ${categoryColors[entry.category]}`}>
                  [{entry.category}]
                </span>
                <span className={categoryColors[entry.category]}>
                  {entry.message}
                </span>
              </div>
            ))}
            <div ref={endRef} />
          </div>
        )}
      </div>
    </div>
  );
}

