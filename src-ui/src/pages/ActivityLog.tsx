import { useRef, useEffect, useState } from "react";
import { useActivityStore } from "../stores/activityStore";
import { Button } from "../components/common/Button";
import { Tabs } from "../components/common/Tabs";
import type { ActivityCategory } from "../types/activity";

type Filter = ActivityCategory | "all";

const filters: { label: string; value: Filter }[] = [
  { label: "All", value: "all" },
  { label: "Actions", value: "action" },
  { label: "Danger", value: "danger" },
  { label: "Warnings", value: "warning" },
  { label: "Errors", value: "error" },
  { label: "System", value: "system" },
];

const categoryColors: Record<string, string> = {
  action: "text-success-text",
  danger: "text-danger-text",
  warning: "text-warning-text",
  system: "text-info-text",
  error: "text-danger-text",
};

export default function ActivityLog() {
  const entries = useActivityStore((s) => s.filteredEntries());
  const filter = useActivityStore((s) => s.filter);
  const setFilter = useActivityStore((s) => s.setFilter);
  const clear = useActivityStore((s) => s.clear);
  const [paused, setPaused] = useState(false);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!paused) {
      endRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [entries.length, paused]);

  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <div className="flex items-center justify-between gap-4">
        <Tabs items={filters} value={filter} onChange={setFilter} />
        <div className="flex items-center gap-2">
          <Button variant="secondary" size="sm" onClick={() => setPaused(!paused)}>
            {paused ? "Resume" : "Pause"}
          </Button>
          <Button variant="danger" size="sm" onClick={clear}>
            Clear
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto rounded-lg border border-border bg-sunken p-4 font-mono text-xs leading-[1.9]">
        {entries.length === 0 ? (
          <p className="text-muted">No activity entries.</p>
        ) : (
          <div>
            {entries.map((entry) => (
              <div
                key={entry.id}
                onClick={() =>
                  setExpandedId(expandedId === entry.id ? null : entry.id)
                }
                className="cursor-pointer rounded-xs px-1 hover:bg-elevated"
              >
                <div className="flex gap-3">
                  <span className="shrink-0 text-muted">{entry.timestamp}</span>
                  <span className="w-16 shrink-0 text-muted">
                    [{entry.category}]
                  </span>
                  <span className={categoryColors[entry.category]}>
                    {entry.message}
                  </span>
                </div>
                {expandedId === entry.id && entry.details && (
                  <div className="pl-[7.5rem] text-subtle">{entry.details}</div>
                )}
              </div>
            ))}
            <div ref={endRef} />
          </div>
        )}
      </div>
    </div>
  );
}
