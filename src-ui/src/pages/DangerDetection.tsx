import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { Slider } from "../components/common/Slider";
import { KeyInput } from "../components/common/KeyInput";
import { TagList } from "../components/common/TagList";
import { useConfigStore } from "../stores/configStore";

export default function DangerDetection() {
  const danger = useConfigStore((s) => s.config.danger_detection);
  const neutral = useConfigStore((s) => s.config.neutral_items);
  const updateDanger = (updates: Partial<typeof danger>) =>
    useConfigStore.getState().updateConfig("danger_detection", updates);
  const updateNeutral = (updates: Partial<typeof neutral>) =>
    useConfigStore.getState().updateConfig("neutral_items", updates);

  return (
    <div className="space-y-6 p-6">
      <h2 className="text-xl font-semibold">Danger Detection</h2>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="Core Settings">
            <Toggle label="Enable Danger Detection" checked={danger.enabled} onChange={(v) => updateDanger({ enabled: v })} />
            <Slider label="HP Threshold" value={danger.hp_threshold_percent} min={30} max={90} onChange={(v) => updateDanger({ hp_threshold_percent: v })} suffix="%" />
            <Slider label="Rapid Loss Threshold" value={danger.rapid_loss_hp} min={50} max={300} onChange={(v) => updateDanger({ rapid_loss_hp: v })} suffix=" HP" />
            <Slider label="Burst Time Window" value={danger.time_window_ms} min={100} max={2000} onChange={(v) => updateDanger({ time_window_ms: v })} suffix="ms" />
            <Slider label="Clear Delay" value={danger.clear_delay_seconds} min={1} max={10} onChange={(v) => updateDanger({ clear_delay_seconds: v })} suffix="s" />
          </Card>

          <Card title="Healing in Danger">
            <Slider label="Healing HP Threshold" value={danger.healing_threshold_in_danger} min={30} max={80} onChange={(v) => updateDanger({ healing_threshold_in_danger: v })} suffix="%" />
            <Slider label="Max Healing Items/Event" value={danger.max_healing_items_per_danger} min={1} max={5} onChange={(v) => updateDanger({ max_healing_items_per_danger: v })} />
            <div className="mt-2 text-xs text-muted">
              <p className="font-medium text-subtle">Priority: Cheese → Greater Faerie Fire → Enchanted Mango → Magic Wand → Faerie Fire</p>
            </div>
          </Card>
        </div>

        <div className="space-y-4">
          <Card title="Defensive Items">
            <Toggle label="Black King Bar" checked={danger.auto_bkb} onChange={(v) => updateDanger({ auto_bkb: v })} />
            <Toggle label="Satanic" checked={danger.auto_satanic} onChange={(v) => updateDanger({ auto_satanic: v })} />
            {danger.auto_satanic && (
              <Slider label="Satanic HP Threshold" value={danger.satanic_hp_threshold} min={10} max={70} onChange={(v) => updateDanger({ satanic_hp_threshold: v })} suffix="%" />
            )}
            <Toggle label="Blade Mail" checked={danger.auto_blade_mail} onChange={(v) => updateDanger({ auto_blade_mail: v })} />
            <Toggle label="Glimmer Cape" checked={danger.auto_glimmer_cape} onChange={(v) => updateDanger({ auto_glimmer_cape: v })} />
            <Toggle label="Ghost Scepter" checked={danger.auto_ghost_scepter} onChange={(v) => updateDanger({ auto_ghost_scepter: v })} />
            <Toggle label="Shiva's Guard" checked={danger.auto_shivas_guard} onChange={(v) => updateDanger({ auto_shivas_guard: v })} />
          </Card>

          <Card title="Dispels">
            <Toggle label="Auto-Manta on Silence" checked={danger.auto_manta_on_silence} onChange={(v) => updateDanger({ auto_manta_on_silence: v })} />
            <Toggle label="Auto-Lotus on Silence" checked={danger.auto_lotus_on_silence} onChange={(v) => updateDanger({ auto_lotus_on_silence: v })} />
          </Card>

          <Card title="Neutral Items" collapsible>
            <Toggle label="Enable" checked={neutral.enabled} onChange={(v) => updateNeutral({ enabled: v })} />
            <Toggle label="Use in Danger Only" checked={neutral.use_in_danger} onChange={(v) => updateNeutral({ use_in_danger: v })} />
            <Slider label="HP Threshold" value={neutral.hp_threshold} min={10} max={90} onChange={(v) => updateNeutral({ hp_threshold: v })} suffix="%" />
            <KeyInput label="Self-Cast Key" value={neutral.self_cast_key} onChange={(v) => updateNeutral({ self_cast_key: v })} />
            <TagList label="Allowed Items" items={neutral.allowed_items} onChange={(v) => updateNeutral({ allowed_items: v })} />
          </Card>
        </div>
      </div>
    </div>
  );
}

