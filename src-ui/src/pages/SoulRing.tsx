import { Badge } from "../components/common/Badge";
import { Card } from "../components/common/Card";
import { Slider } from "../components/common/Slider";
import { NumberInput } from "../components/common/NumberInput";
import { SettingRow } from "../components/common/SettingRow";
import { useConfigStore } from "../stores/configStore";

export default function SoulRing() {
  const config = useConfigStore((s) => s.config.soul_ring);
  const update = (updates: Partial<typeof config>) =>
    useConfigStore.getState().updateConfig("soul_ring", updates);

  return (
    <div className="grid grid-cols-1 items-start gap-4 p-6 lg:grid-cols-2">
      <Card title="Settings">
        <SettingRow
          label="Enable Soul Ring"
          checked={config.enabled}
          onChange={(v) => update({ enabled: v })}
        />
        <Slider
          label="Min Mana to Trigger"
          value={config.min_mana_percent}
          min={0}
          max={100}
          onChange={(v) => update({ min_mana_percent: v })}
          suffix="%"
        />
        <Slider
          label="Min Health Safety Floor"
          value={config.min_health_percent}
          min={0}
          max={50}
          onChange={(v) => update({ min_health_percent: v })}
          suffix="%"
        />
        <NumberInput
          label="Delay Before Ability"
          value={config.delay_before_ability_ms}
          onChange={(v) => update({ delay_before_ability_ms: v })}
          suffix="ms"
        />
        <NumberInput
          label="Trigger Cooldown"
          value={config.trigger_cooldown_ms}
          onChange={(v) => update({ trigger_cooldown_ms: v })}
          suffix="ms"
        />
      </Card>

      <Card title="Intercepted Keys">
        <div className="flex flex-wrap gap-2">
          {config.ability_keys.map((key) => (
            <Badge key={key} square outline className="h-7 w-7 justify-center px-0 text-sm">
              {key.toUpperCase()}
            </Badge>
          ))}
        </div>
        <SettingRow
          label="Intercept Item Keys"
          checked={config.intercept_item_keys}
          onChange={(v) => update({ intercept_item_keys: v })}
        />
        <p className="text-xs leading-relaxed text-muted">
          Soul Ring pre-casts before these keys when mana is below threshold.
          Excludes Blink, TP, BKB, Armlet, and consumables.
        </p>
      </Card>
    </div>
  );
}
