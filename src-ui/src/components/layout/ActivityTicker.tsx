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
  action: "text-terminal",
  danger: "text-danger",
  warning: "text-warning",
  system: "text-info",
};

export function ActivityTicker({ entries }: ActivityTickerProps) {
  const [expanded, setExpanded] = useState(false);
  const visible = expanded ? entries.slice(-3) : entries.slice(-1);

  return (
    <div className="shrink-0 border-t border-border bg-terminal-bg px-4 py-1">
      <div className="flex items-center justify-between">
        <div className="flex-1 overflow-hidden">
          {visible.map((entry) => (
            <div key={entry.id} className="flex items-center gap-2 font-mono text-xs">
              <span className="text-muted">{entry.timestamp}</span>
              <span className={categoryColors[entry.category]}>{entry.message}</span>
            </div>
          ))}
        </div>
        <div className="flex items-center gap-2 ml-2">
          <Link to="/activity" className="text-xs text-gold hover:underline">
            View All
          </Link>
          <button
            type="button"
            onClick={() => setExpanded(!expanded)}
            className="text-muted hover:text-content"
          >
            {expanded ? <ChevronDown className="h-3 w-3" /> : <ChevronUp className="h-3 w-3" />}
          </button>
        </div>
      </div>
    </div>
  );
}
