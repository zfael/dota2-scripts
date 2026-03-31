import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { Slider } from "../components/common/Slider";
import { NumberInput } from "../components/common/NumberInput";
import { useConfigStore } from "../stores/configStore";

export default function SoulRing() {
  const config = useConfigStore((s) => s.config.soul_ring);
  const update = (updates: Partial<typeof config>) =>
    useConfigStore.getState().updateConfig("soul_ring", updates);

  return (
    <div className="space-y-6 p-6">
      <h2 className="text-xl font-semibold">Soul Ring</h2>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="Settings">
            <Toggle label="Enable Soul Ring" checked={config.enabled} onChange={(v) => update({ enabled: v })} />
            <Slider label="Min Mana to Trigger" value={config.min_mana_percent} min={0} max={100} onChange={(v) => update({ min_mana_percent: v })} suffix="%" />
            <Slider label="Min Health Safety Floor" value={config.min_health_percent} min={0} max={50} onChange={(v) => update({ min_health_percent: v })} suffix="%" />
            <NumberInput label="Delay Before Ability" value={config.delay_before_ability_ms} onChange={(v) => update({ delay_before_ability_ms: v })} suffix="ms" />
            <NumberInput label="Trigger Cooldown" value={config.trigger_cooldown_ms} onChange={(v) => update({ trigger_cooldown_ms: v })} suffix="ms" />
          </Card>
        </div>

        <div className="space-y-4">
          <Card title="Intercepted Keys">
            <div className="flex flex-wrap gap-2">
              {config.ability_keys.map((key) => (
                <span
                  key={key}
                  className="flex h-8 w-8 items-center justify-center rounded-md border border-border bg-elevated font-mono text-sm font-semibold text-gold"
                >
                  {key.toUpperCase()}
                </span>
              ))}
            </div>
            <Toggle
              label="Intercept Item Keys"
              checked={config.intercept_item_keys}
              onChange={(v) => update({ intercept_item_keys: v })}
            />
            <p className="text-xs text-muted">
              Soul Ring pre-casts before these keys when mana is below threshold.
              Excludes Blink, TP, BKB, Armlet, and consumables.
            </p>
          </Card>
        </div>
      </div>
    </div>
  );
}

