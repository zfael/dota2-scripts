import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { Slider } from "../../common/Slider";
import { NumberInput } from "../../common/NumberInput";
import { KeyInput } from "../../common/KeyInput";
import { TagList } from "../../common/TagList";
import { useConfigStore } from "../../../stores/configStore";

export default function OutworldDestroyerConfig() {
  const config = useConfigStore((s) => s.config.heroes.outworld_destroyer);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("outworld_destroyer", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Keybindings">
          <div className="grid grid-cols-2 gap-3">
            <KeyInput label="Combo Key" value={config.standalone_key} onChange={(v) => set({ standalone_key: v })} />
            <KeyInput label="Objurgation" value={config.objurgation_key} onChange={(v) => set({ objurgation_key: v })} />
            <KeyInput label="Arcane Orb" value={config.arcane_orb_key} onChange={(v) => set({ arcane_orb_key: v })} />
            <KeyInput label="Astral Imprisonment" value={config.astral_imprisonment_key} onChange={(v) => set({ astral_imprisonment_key: v })} />
          </div>
        </Card>

        <Card title="Auto-Objurgation on Danger">
          <Toggle label="Enable" checked={config.auto_objurgation_on_danger} onChange={(v) => set({ auto_objurgation_on_danger: v })} />
          <Slider label="HP Threshold" value={config.objurgation_hp_threshold_percent} min={10} max={90} onChange={(v) => set({ objurgation_hp_threshold_percent: v })} suffix="%" />
          <Slider label="Min Mana" value={config.objurgation_min_mana_percent} min={0} max={100} onChange={(v) => set({ objurgation_min_mana_percent: v })} suffix="%" />
          <NumberInput label="Trigger Cooldown" value={config.objurgation_trigger_cooldown_ms} onChange={(v) => set({ objurgation_trigger_cooldown_ms: v })} suffix="ms" />
        </Card>

        <Card title="Ultimate Intercept">
          <Toggle label="Enable" checked={config.ultimate_intercept_enabled} onChange={(v) => set({ ultimate_intercept_enabled: v })} />
          <Toggle label="Auto-BKB on Ultimate" checked={config.auto_bkb_on_ultimate} onChange={(v) => set({ auto_bkb_on_ultimate: v })} />
          <Toggle label="Auto-Objurgation on Ultimate" checked={config.auto_objurgation_on_ultimate} onChange={(v) => set({ auto_objurgation_on_ultimate: v })} />
          <NumberInput label="Post-BKB Delay" value={config.post_bkb_delay_ms} onChange={(v) => set({ post_bkb_delay_ms: v })} suffix="ms" />
          <NumberInput label="Post-Blink Delay" value={config.post_blink_delay_ms} onChange={(v) => set({ post_blink_delay_ms: v })} suffix="ms" />
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Standalone Combo">
          <TagList label="Combo Items" items={config.combo_items} onChange={(v) => set({ combo_items: v })} />
          <div className="grid grid-cols-2 gap-3">
            <NumberInput label="Item Spam Count" value={config.combo_item_spam_count} onChange={(v) => set({ combo_item_spam_count: v })} />
            <NumberInput label="Item Delay" value={config.combo_item_delay_ms} onChange={(v) => set({ combo_item_delay_ms: v })} suffix="ms" />
            <NumberInput label="Post-Ult Orb Presses" value={config.post_ultimate_arcane_orb_presses} onChange={(v) => set({ post_ultimate_arcane_orb_presses: v })} />
            <NumberInput label="Orb Press Interval" value={config.arcane_orb_press_interval_ms} onChange={(v) => set({ arcane_orb_press_interval_ms: v })} suffix="ms" />
          </div>
        </Card>

        <Card title="Self-Astral Panic" collapsible>
          <Toggle label="Enable" checked={config.astral_self_cast_enabled} onChange={(v) => set({ astral_self_cast_enabled: v })} />
          <KeyInput label="Panic Key" value={config.astral_self_cast_key} onChange={(v) => set({ astral_self_cast_key: v })} />
        </Card>
      </div>
    </>
  );
}

