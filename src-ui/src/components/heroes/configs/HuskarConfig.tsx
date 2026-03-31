import { Card } from "../../common/Card";
import { KeyInput } from "../../common/KeyInput";
import { NumberInput } from "../../common/NumberInput";
import { useConfigStore } from "../../../stores/configStore";

export default function HuskarConfig() {
  const config = useConfigStore((s) => s.config.heroes.huskar);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("huskar", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Keybindings">
          <KeyInput label="Standalone Key" value={config.standalone_key} onChange={(v) => set({ standalone_key: v })} />
          <KeyInput label="Berserker Blood Key" value={config.berserker_blood_key} onChange={(v) => set({ berserker_blood_key: v })} />
        </Card>

        <Card title="Berserker Blood">
          <NumberInput label="Cleanse Delay" value={config.berserker_blood_delay_ms} onChange={(v) => set({ berserker_blood_delay_ms: v })} suffix="ms" />
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Armlet Override">
          <NumberInput label="Toggle Threshold" value={config.armlet_toggle_threshold} onChange={(v) => set({ armlet_toggle_threshold: v })} suffix="HP" />
          <NumberInput label="Predictive Offset" value={config.armlet_predictive_offset} onChange={(v) => set({ armlet_predictive_offset: v })} suffix="HP" />
          <NumberInput label="Toggle Cooldown" value={config.armlet_toggle_cooldown_ms} onChange={(v) => set({ armlet_toggle_cooldown_ms: v })} suffix="ms" />
        </Card>
      </div>
    </>
  );
}

