import { Toggle } from "../components/common/Toggle";
import { Card } from "../components/common/Card";
import { useUIStore } from "../stores/uiStore";
import { useGameStore } from "../stores/gameStore";
import { useActivityStore } from "../stores/activityStore";
import { HEROES } from "../types/game";
import { Link } from "react-router-dom";

export default function Dashboard() {
  const gsiEnabled = useUIStore((s) => s.gsiEnabled);
  const setGsiEnabled = useUIStore((s) => s.setGsiEnabled);
  const standaloneEnabled = useUIStore((s) => s.standaloneEnabled);
  const setStandaloneEnabled = useUIStore((s) => s.setStandaloneEnabled);
  const heroName = useGameStore((s) => s.game.heroName);
  const entries = useActivityStore((s) => s.entries);

  const activeHero = HEROES.find(
    (h) => h.displayName.toLowerCase() === heroName?.toLowerCase(),
  );

  const recentEntries = entries.slice(-5);

  const categoryColor: Record<string, string> = {
    action: "text-terminal",
    danger: "text-danger",
    warning: "text-warning",
    system: "text-info",
    error: "text-danger",
  };

  return (
    <div className="space-y-6 p-6">
      <h2 className="text-xl font-semibold">Dashboard</h2>

      <Card title="Quick Controls">
        <div className="space-y-3">
          <Toggle
            label="GSI Automation"
            checked={gsiEnabled}
            onChange={setGsiEnabled}
          />
          <Toggle
            label="Standalone Script"
            checked={standaloneEnabled}
            onChange={setStandaloneEnabled}
          />
        </div>
      </Card>

      <Card title="Active Hero">
        {activeHero ? (
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <span className="text-3xl">{activeHero.icon}</span>
              <div>
                <p className="font-semibold text-content">
                  {activeHero.displayName}
                </p>
                <p className="text-xs text-subtle">{activeHero.role}</p>
              </div>
            </div>
            <Link
              to={`/heroes/${activeHero.id}`}
              className="text-sm text-gold hover:underline"
            >
              View Config →
            </Link>
          </div>
        ) : (
          <div className="grid grid-cols-4 gap-2">
            <button
              type="button"
              onClick={() => useGameStore.getState().setGame({ heroName: null })}
              className="flex flex-col items-center gap-1 rounded-md border border-border p-2 text-center hover:bg-elevated transition-colors"
            >
              <span className="text-xl">🚫</span>
              <span className="text-xs text-subtle">None</span>
            </button>
            {HEROES.map((hero) => (
              <Link
                key={hero.id}
                to={`/heroes/${hero.id}`}
                className="flex flex-col items-center gap-1 rounded-md border border-border p-2 text-center hover:bg-elevated transition-colors"
              >
                <span className="text-xl">{hero.icon}</span>
                <span className="text-xs text-subtle">{hero.displayName}</span>
              </Link>
            ))}
          </div>
        )}
      </Card>

      <Card title="Recent Activity">
        <div className="space-y-1 rounded-md bg-terminal-bg p-3 font-mono text-xs">
          {recentEntries.length === 0 ? (
            <p className="text-muted">No activity yet...</p>
          ) : (
            recentEntries.map((entry) => (
              <div key={entry.id} className="flex gap-2">
                <span className="text-muted shrink-0">{entry.timestamp}</span>
                <span className={categoryColor[entry.category] ?? "text-content"}>
                  {entry.message}
                </span>
              </div>
            ))
          )}
        </div>
        <Link
          to="/activity"
          className="mt-2 inline-block text-xs text-gold hover:underline"
        >
          View Full Log →
        </Link>
      </Card>
    </div>
  );
}
