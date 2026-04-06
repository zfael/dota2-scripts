import { Card } from "../../common/Card";
import { KeyInput } from "../../common/KeyInput";
import { NumberInput } from "../../common/NumberInput";
import { Toggle } from "../../common/Toggle";
import { useConfigStore } from "../../../stores/configStore";

export default function HuskarConfig() {
  const config = useConfigStore((s) => s.config.heroes.huskar);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("huskar", updates);
  const setRoshanSpears = (updates: Partial<typeof config.roshan_spears>) =>
    set({
      roshan_spears: {
        ...config.roshan_spears,
        ...updates,
      },
    });

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

        <Card title="Roshan Spears">
          <Toggle
            label="Enable Roshan Spears Gate"
            checked={config.roshan_spears.enabled}
            onChange={(value) => setRoshanSpears({ enabled: value })}
          />
          <KeyInput
            label="Burning Spears Key"
            value={config.roshan_spears.burning_spear_key}
            onChange={(value) => setRoshanSpears({ burning_spear_key: value })}
          />
          <NumberInput
            label="Disable Buffer"
            value={config.roshan_spears.disable_buffer_hp}
            onChange={(value) => setRoshanSpears({ disable_buffer_hp: value })}
            suffix="HP"
          />
          <NumberInput
            label="Re-enable Buffer"
            value={config.roshan_spears.reenable_buffer_hp}
            onChange={(value) => setRoshanSpears({ reenable_buffer_hp: value })}
            suffix="HP"
          />
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

