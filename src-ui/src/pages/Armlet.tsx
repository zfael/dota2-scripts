import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { NumberInput } from "../components/common/NumberInput";
import { Dropdown } from "../components/common/Dropdown";
import { useConfigStore } from "../stores/configStore";
import { HEROES } from "../types/game";
import { Link } from "react-router-dom";

export default function Armlet() {
  const config = useConfigStore((s) => s.config.armlet);
  const heroes = useConfigStore((s) => s.config.heroes);
  const update = (updates: Partial<typeof config>) =>
    useConfigStore.getState().updateConfig("armlet", updates);

  const heroesWithOverrides = HEROES.filter((h) => {
    const heroConfig = heroes[h.id as keyof typeof heroes];
    return heroConfig && "armlet" in heroConfig;
  });

  return (
    <div className="space-y-6 p-6">
      <h2 className="text-xl font-semibold">Armlet</h2>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="Shared Settings">
            <Toggle label="Enable Armlet" checked={config.enabled} onChange={(v) => update({ enabled: v })} />
            <Dropdown
              label="Cast Modifier"
              value={config.cast_modifier}
              options={[
                { value: "Alt", label: "Alt" },
                { value: "Ctrl", label: "Ctrl" },
                { value: "Shift", label: "Shift" },
              ]}
              onChange={(v) => update({ cast_modifier: v })}
            />
            <NumberInput label="Toggle Threshold" value={config.toggle_threshold} onChange={(v) => update({ toggle_threshold: v })} suffix="HP" />
            <NumberInput label="Predictive Offset" value={config.predictive_offset} onChange={(v) => update({ predictive_offset: v })} suffix="HP" />
            <NumberInput label="Toggle Cooldown" value={config.toggle_cooldown_ms} onChange={(v) => update({ toggle_cooldown_ms: v })} suffix="ms" />
          </Card>
        </div>

        <div className="space-y-4">
          <Card title="Per-Hero Overrides">
            <div className="space-y-2">
              {heroesWithOverrides.map((hero) => (
                <Link
                  key={hero.id}
                  to={`/heroes/${hero.id}`}
                  className="flex items-center justify-between rounded-md border border-border bg-base p-3 transition-colors hover:bg-elevated"
                >
                  <div className="flex items-center gap-2">
                    <span className="text-lg">{hero.icon}</span>
                    <span className="text-sm text-content">{hero.displayName}</span>
                  </div>
                  <span className="text-xs text-gold">Configure →</span>
                </Link>
              ))}
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}

