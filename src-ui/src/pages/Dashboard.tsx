import { Link, useNavigate } from "react-router-dom";
import { Badge } from "../components/common/Badge";
import { Button } from "../components/common/Button";
import { Card } from "../components/common/Card";
import { Avatar } from "../components/common/Avatar";
import { Divider } from "../components/common/Field";
import { SettingRow } from "../components/common/SettingRow";
import { useUIStore } from "../stores/uiStore";
import { useGameStore } from "../stores/gameStore";
import { useConfigStore } from "../stores/configStore";
import { useActivityStore } from "../stores/activityStore";
import { HEROES } from "../types/game";

const categoryColor: Record<string, string> = {
  action: "text-success-text",
  danger: "text-danger-text",
  warning: "text-warning-text",
  system: "text-info-text",
  error: "text-danger-text",
};

export default function Dashboard() {
  const navigate = useNavigate();
  const gsiEnabled = useUIStore((s) => s.gsiEnabled);
  const setGsiEnabled = useUIStore((s) => s.setGsiEnabled);
  const standaloneEnabled = useUIStore((s) => s.standaloneEnabled);
  const setStandaloneEnabled = useUIStore((s) => s.setStandaloneEnabled);
  const heroName = useGameStore((s) => s.game.heroName);
  const connected = useGameStore((s) => s.game.connected);
  const entries = useActivityStore((s) => s.entries);
  const config = useConfigStore((s) => s.config);

  const activeHero = HEROES.find(
    (h) => h.displayName.toLowerCase() === heroName?.toLowerCase(),
  );

  const recentEntries = entries.slice(-5);

  // "Armed" is what the runtime would actually act on, so each tile reads the
  // same switch its page owns rather than a summary the two could drift apart on.
  const modules = [
    { name: "Survivability", to: "/survivability", on: gsiEnabled },
    { name: "Danger", to: "/danger", on: config.danger_detection.enabled },
    { name: "Soul Ring", to: "/soul-ring", on: config.soul_ring.enabled },
    { name: "Armlet", to: "/armlet", on: config.armlet.enabled },
    { name: "Boots", to: "/boots", on: config.phase_boots_automation.enabled },
    { name: "Minimap", to: "/minimap", on: config.minimap_capture.enabled },
    { name: "Waves", to: "/waves", on: config.wave_tracker.enabled },
    { name: "Alerts", to: "/alerts", on: config.alerts.enabled },
  ];

  return (
    <div className="space-y-4 p-6">
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[1.1fr_1fr]">
        <Card title="Runtime" subtitle="Master switches for the two automation paths">
          <SettingRow
            label="GSI Automation"
            description="Reacts to live game state from Dota"
            checked={gsiEnabled}
            onChange={setGsiEnabled}
          />
          <Divider />
          <SettingRow
            label="Standalone Script"
            description="Keyboard combos, no game state needed"
            checked={standaloneEnabled}
            onChange={setStandaloneEnabled}
          />
        </Card>

        <Card
          title="Active Hero"
          subtitle="Detected from GSI, override if Dota reports the wrong one"
        >
          <div className="flex items-center gap-3">
            <Avatar
              name={activeHero?.displayName ?? "No hero"}
              glyph={activeHero?.icon ?? "🚫"}
              size="lg"
              status={connected ? "online" : "offline"}
            />
            <div className="min-w-0 flex-1">
              <div className="text-base font-semibold text-content">
                {activeHero?.displayName ?? "No hero"}
              </div>
              <div className="text-xs text-muted">
                {activeHero?.role ?? "Waiting for game"}
              </div>
            </div>
            {activeHero && (
              <Button
                variant="soft"
                size="sm"
                onClick={() => navigate(`/heroes/${activeHero.id}`)}
              >
                Open config
              </Button>
            )}
          </div>

          <div className="flex items-center gap-3">
            <span className="shrink-0 text-xs whitespace-nowrap text-muted">
              Manual override
            </span>
            <select
              aria-label="Manual hero override"
              value={activeHero?.displayName ?? ""}
              onChange={(e) =>
                useGameStore
                  .getState()
                  .setGame({ heroName: e.target.value || null })
              }
              className="h-9 w-full rounded-md border border-border bg-input px-3 text-sm text-content transition-colors hover:border-border-strong focus:border-accent focus:outline-none"
            >
              <option value="">No hero</option>
              {HEROES.map((hero) => (
                <option key={hero.id} value={hero.displayName}>
                  {hero.displayName}
                </option>
              ))}
            </select>
          </div>
        </Card>
      </div>

      <Card title="Automation Modules" subtitle="What is armed right now" flushBody>
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          {modules.map((module) => (
            <Link
              key={module.to}
              to={module.to}
              className="flex cursor-pointer flex-col items-start gap-2 rounded-md border border-border bg-elevated p-3 text-sm text-content transition-colors hover:border-border-strong hover:bg-raised"
            >
              <span className="font-medium">{module.name}</span>
              <Badge tone={module.on ? "success" : "neutral"} dot>
                {module.on ? "Armed" : "Off"}
              </Badge>
            </Link>
          ))}
        </div>
      </Card>

      <Card
        title="Recent Activity"
        flushBody
        footer={
          <Link to="/activity">
            <Button variant="ghost" size="sm">
              Full log
            </Button>
          </Link>
        }
      >
        <div className="flex flex-col gap-1.5 font-mono text-xs">
          {recentEntries.length === 0 ? (
            <p className="text-muted">No activity yet...</p>
          ) : (
            recentEntries.map((entry) => (
              <div key={entry.id} className="flex gap-3">
                <span className="shrink-0 text-muted">{entry.timestamp}</span>
                <span className="w-16 shrink-0 text-muted">[{entry.category}]</span>
                <span className={categoryColor[entry.category] ?? "text-content"}>
                  {entry.message}
                </span>
              </div>
            ))
          )}
        </div>
      </Card>
    </div>
  );
}
