import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { Slider } from "../../common/Slider";
import { NumberInput } from "../../common/NumberInput";
import { KeyInput } from "../../common/KeyInput";
import { useConfigStore } from "../../../stores/configStore";

export default function LargoConfig() {
  const config = useConfigStore((s) => s.config.heroes.largo);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("largo", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Keybindings">
          <div className="grid grid-cols-2 gap-3">
            <KeyInput label="Q Ability" value={config.q_ability_key} onChange={(v) => set({ q_ability_key: v })} />
            <KeyInput label="W Ability" value={config.w_ability_key} onChange={(v) => set({ w_ability_key: v })} />
            <KeyInput label="E Ability" value={config.e_ability_key} onChange={(v) => set({ e_ability_key: v })} />
            <KeyInput label="R Ability" value={config.r_ability_key} onChange={(v) => set({ r_ability_key: v })} />
          </div>
        </Card>

        <Card title="Amphibian Rhapsody">
          <Toggle label="Enable" checked={config.amphibian_rhapsody_enabled} onChange={(v) => set({ amphibian_rhapsody_enabled: v })} />
          <NumberInput label="Beat Interval" value={config.beat_interval_ms} onChange={(v) => set({ beat_interval_ms: v })} suffix="ms" />
          <NumberInput label="Beat Correction" value={config.beat_correction_ms} onChange={(v) => set({ beat_correction_ms: v })} suffix="ms" />
          <NumberInput label="Correct Every N Beats" value={config.beat_correction_every_n_beats} onChange={(v) => set({ beat_correction_every_n_beats: v })} />
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Auto Behavior">
          <Toggle label="Auto Toggle on Danger" checked={config.auto_toggle_on_danger} onChange={(v) => set({ auto_toggle_on_danger: v })} />
          <Slider label="Mana Threshold" value={config.mana_threshold_percent} min={0} max={100} onChange={(v) => set({ mana_threshold_percent: v })} suffix="%" />
          <Slider label="Heal HP Threshold" value={config.heal_hp_threshold} min={0} max={100} onChange={(v) => set({ heal_hp_threshold: v })} suffix="%" />
        </Card>
      </div>
    </>
  );
}

