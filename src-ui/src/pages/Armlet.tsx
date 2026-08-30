import { Link } from "react-router-dom";
import { Avatar } from "../components/common/Avatar";
import { Badge } from "../components/common/Badge";
import { Card } from "../components/common/Card";
import { Dropdown } from "../components/common/Dropdown";
import { KeyInput } from "../components/common/KeyInput";
import { NumberInput } from "../components/common/NumberInput";
import { SettingRow } from "../components/common/SettingRow";
import { useConfigStore } from "../stores/configStore";
import { useUIStore } from "../stores/uiStore";
import { HEROES } from "../types/game";

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
    <div className="grid grid-cols-1 items-start gap-4 p-6 lg:grid-cols-2">
      <div className="space-y-4">
        <Card title="Shared Settings">
          <SettingRow
            label="Enable Armlet"
            checked={config.enabled}
            onChange={(v) => update({ enabled: v })}
          />
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
          <NumberInput
            label="Toggle Threshold"
            value={config.toggle_threshold}
            onChange={(v) => update({ toggle_threshold: v })}
            suffix="HP"
          />
          <NumberInput
            label="Predictive Offset"
            value={config.predictive_offset}
            onChange={(v) => update({ predictive_offset: v })}
            suffix="HP"
          />
          <NumberInput
            label="Toggle Cooldown"
            value={config.toggle_cooldown_ms}
            onChange={(v) => update({ toggle_cooldown_ms: v })}
            suffix="ms"
          />
        </Card>

        <Card title="Roshan Mode" subtitle="Extra protection while tanking Roshan">
          <SettingRow
            label="Enable Roshan Protection"
            checked={config.roshan.enabled}
            onChange={(v) => updateRoshan({ enabled: v })}
          />
          <KeyInput
            label="Roshan Toggle Key"
            value={config.roshan.toggle_key}
            onChange={(value) => updateRoshan({ toggle_key: value })}
          />
          <div className="flex items-center justify-between gap-3 rounded-md border border-border bg-sunken p-3">
            <span className="text-sm text-subtle">Current Status</span>
            <Badge tone={armletRoshanArmed ? "warning" : "neutral"} dot>
              {armletRoshanArmed ? "Armed" : "Disarmed"}
            </Badge>
          </div>
          <SettingRow
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
        </Card>

        {/* Damage-model tuning: separated so the card above stays the thing you
            actually touch between games. */}
        <Card title="Roshan Damage Learning" collapsible defaultOpen={false}>
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

      <Card
        title="Per-Hero Overrides"
        subtitle="Heroes that change the shared Armlet behaviour"
        flushBody
      >
        <div className="flex flex-col gap-1.5">
          {heroesWithOverrides.map((hero) => (
            <Link
              key={hero.id}
              to={`/heroes/${hero.id}`}
              className="flex items-center gap-3 rounded-md border border-border bg-elevated p-3 text-sm text-content transition-colors hover:bg-raised"
            >
              <Avatar name={hero.displayName} glyph={hero.icon} size="sm" />
              <span className="flex-1 text-left">{hero.displayName}</span>
              <span className="text-accent-text">Configure →</span>
            </Link>
          ))}
        </div>
      </Card>
    </div>
  );
}
