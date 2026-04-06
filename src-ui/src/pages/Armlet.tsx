import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { NumberInput } from "../components/common/NumberInput";
import { Dropdown } from "../components/common/Dropdown";
import { KeyInput } from "../components/common/KeyInput";
import { useConfigStore } from "../stores/configStore";
import { useUIStore } from "../stores/uiStore";
import { HEROES } from "../types/game";
import { Link } from "react-router-dom";

export default function Armlet() {
  const config = useConfigStore((s) => s.config.armlet);
  const heroes = useConfigStore((s) => s.config.heroes);
  const armletRoshanArmed = useUIStore((s) => s.armletRoshanArmed);
  const setArmletRoshanArmed = useUIStore((s) => s.setArmletRoshanArmed);
  const update = (updates: Partial<typeof config>) =>
    useConfigStore.getState().updateConfig("armlet", updates);
  const updateRoshan = (updates: Partial<typeof config.roshan>) =>
    update({ roshan: { ...config.roshan, ...updates } });

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

          <Card title="Roshan Mode">
            <Toggle
              label="Enable Roshan Protection"
              checked={config.roshan.enabled}
              onChange={(v) => updateRoshan({ enabled: v })}
            />
            <KeyInput
              label="Roshan Toggle Key"
              value={config.roshan.toggle_key}
              onChange={(value) => updateRoshan({ toggle_key: value })}
            />
            <div className="rounded-md border border-border bg-base px-3 py-2">
              <div className="flex items-center justify-between gap-3">
                <span className="text-sm text-subtle">Current Status</span>
                <span className={`text-sm font-medium ${armletRoshanArmed ? "text-gold" : "text-subtle"}`}>
                  {armletRoshanArmed ? "Armed" : "Disarmed"}
                </span>
              </div>
            </div>
            <Toggle
              label="Arm Roshan Mode"
              checked={armletRoshanArmed}
              onChange={setArmletRoshanArmed}
              disabled={!config.roshan.enabled}
            />
            <NumberInput
              label="Emergency Margin"
              value={config.roshan.emergency_margin_hp}
              onChange={(v) => updateRoshan({ emergency_margin_hp: v })}
              suffix="HP"
            />
            <NumberInput
              label="Learning Window"
              value={config.roshan.learning_window_ms}
              onChange={(v) => updateRoshan({ learning_window_ms: v })}
              suffix="ms"
            />
            <NumberInput
              label="Confidence Hits"
              value={config.roshan.min_confidence_hits}
              onChange={(v) => updateRoshan({ min_confidence_hits: v })}
            />
            <NumberInput
              label="Minimum Sample Damage"
              value={config.roshan.min_sample_damage}
              onChange={(v) => updateRoshan({ min_sample_damage: v })}
              suffix="HP"
            />
            <NumberInput
              label="Stale Reset"
              value={config.roshan.stale_reset_ms}
              onChange={(v) => updateRoshan({ stale_reset_ms: v })}
              suffix="ms"
            />
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

