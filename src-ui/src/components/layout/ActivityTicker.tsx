import { useState } from "react";
import { Link } from "react-router-dom";
import { ChevronUp, ChevronDown } from "lucide-react";

export interface TickerEntry {
  id: string;
  timestamp: string;
  category: "action" | "danger" | "warning" | "system";
  message: string;
}

interface ActivityTickerProps {
  entries: TickerEntry[];
}

const categoryColors: Record<string, string> = {
  action: "text-success-text",
  danger: "text-danger-text",
  warning: "text-warning-text",
  system: "text-info-text",
};

export function ActivityTicker({ entries }: ActivityTickerProps) {
  const [expanded, setExpanded] = useState(false);
  const visible = expanded ? entries.slice(-3) : entries.slice(-1);

  // An empty strip is just a stray "View all" pinned to the bottom of the app.
  if (entries.length === 0) return null;

  return (
    <div
      className={`flex shrink-0 items-center gap-4 border-t border-border bg-sunken px-5 font-mono text-2xs ${
        expanded ? "py-2" : "h-[34px]"
      }`}
    >
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        {visible.map((entry) => (
          <div key={entry.id} className="flex items-center gap-4">
            <span className="shrink-0 text-muted">{entry.timestamp}</span>
            <span className={`truncate ${categoryColors[entry.category]}`}>
              {entry.message}
            </span>
          </div>
        ))}
      </div>
      <div className="flex shrink-0 items-center gap-3">
        <Link to="/activity" className="text-accent-text hover:underline">
          View all
        </Link>
        <button
          type="button"
          onClick={() => setExpanded(!expanded)}
          aria-label={expanded ? "Collapse ticker" : "Expand ticker"}
          className="cursor-pointer text-muted hover:text-content"
        >
          {expanded ? (
            <ChevronDown className="h-3 w-3" />
          ) : (
            <ChevronUp className="h-3 w-3" />
          )}
        </button>
      </div>
    </div>
  );
}
