import { useEffect, useState } from "react";
import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { Slider } from "../../common/Slider";
import { NumberInput } from "../../common/NumberInput";
import { KeyInput } from "../../common/KeyInput";
import { TagList } from "../../common/TagList";
import { useConfigStore } from "../../../stores/configStore";
import { isTauri } from "../../../lib/tauri";
import type { MeepoObservedState } from "../../../types/game";

export default function MeepoConfig() {
  const config = useConfigStore((s) => s.config.heroes.meepo);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("meepo", updates);
  const setFarm = (updates: Partial<typeof config.farm_assist>) =>
    set({ farm_assist: { ...config.farm_assist, ...updates } });

  const [meepoState, setMeepoState] = useState<MeepoObservedState | null>(null);

  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;

    const poll = async () => {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        while (!cancelled) {
          const state = await invoke<MeepoObservedState | null>("get_meepo_state");
          if (!cancelled) setMeepoState(state);
          await new Promise((r) => setTimeout(r, 500));
        }
      } catch {
        // Silently ignore — command may not be available
      }
    };

    poll();
    return () => { cancelled = true; };
  }, []);

  return (
    <>
      {meepoState && (
        <div className="col-span-2">
          <Card title="Live State">
            <div className="grid grid-cols-3 gap-3 text-sm">
              <div>
                <span className="text-muted">HP:</span>{" "}
                <span className={meepoState.healthPercent < 30 ? "text-danger" : "text-terminal"}>
                  {meepoState.healthPercent}%
                </span>
              </div>
              <div>
                <span className="text-muted">Mana:</span>{" "}
                <span className="text-info">{meepoState.manaPercent}%</span>
              </div>
              <div>
                <span className="text-muted">Status:</span>{" "}
                {meepoState.inDanger ? (
                  <span className="text-danger">⚠ DANGER</span>
                ) : meepoState.alive ? (
                  <span className="text-terminal">Alive</span>
                ) : (
                  <span className="text-muted">Dead</span>
                )}
              </div>
              <div>
                <span className="text-muted">Poof:</span>{" "}
                <span className={meepoState.poofReady ? "text-terminal" : "text-muted"}>
                  {meepoState.poofReady ? "Ready" : "CD"}
                </span>
              </div>
              <div>
                <span className="text-muted">Dig:</span>{" "}
                <span className={meepoState.digReady ? "text-terminal" : "text-muted"}>
                  {meepoState.digReady ? "Ready" : "CD"}
                </span>
              </div>
              <div>
                <span className="text-muted">MegaMeepo:</span>{" "}
                <span className={meepoState.megameepoReady ? "text-terminal" : "text-muted"}>
                  {meepoState.megameepoReady ? "Ready" : "CD"}
                </span>
              </div>
              <div>
                <span className="text-muted">Blink:</span>{" "}
                <span className={meepoState.blinkAvailable ? "text-terminal" : "text-muted"}>
                  {meepoState.blinkAvailable ? "Available" : "No"}
                </span>
              </div>
              <div>
                <span className="text-muted">Shard:</span>{" "}
                {meepoState.hasShard ? "✓" : "✗"}
              </div>
              <div>
                <span className="text-muted">Scepter:</span>{" "}
                {meepoState.hasScepter ? "✓" : "✗"}
              </div>
            </div>
            {meepoState.comboItems.length > 0 && (
              <div className="mt-2 text-sm">
                <span className="text-muted">Combo items ready:</span>{" "}
                {meepoState.comboItems.join(", ")}
              </div>
            )}
          </Card>
        </div>
      )}

      <div className="space-y-4">
        <Card title="Keybindings">
          <div className="grid grid-cols-2 gap-3">
            <KeyInput label="Combo Key" value={config.standalone_key} onChange={(v) => set({ standalone_key: v })} />
            <KeyInput label="Earthbind" value={config.earthbind_key} onChange={(v) => set({ earthbind_key: v })} />
            <KeyInput label="Poof" value={config.poof_key} onChange={(v) => set({ poof_key: v })} />
            <KeyInput label="Dig" value={config.dig_key} onChange={(v) => set({ dig_key: v })} />
            <KeyInput label="MegaMeepo" value={config.megameepo_key} onChange={(v) => set({ megameepo_key: v })} />
          </div>
        </Card>

        <Card title="Combo Settings">
          <NumberInput label="Post-Blink Delay" value={config.post_blink_delay_ms} onChange={(v) => set({ post_blink_delay_ms: v })} suffix="ms" />
          <TagList label="Combo Items" items={config.combo_items} onChange={(v) => set({ combo_items: v })} />
          <div className="grid grid-cols-2 gap-3">
            <NumberInput label="Item Spam Count" value={config.combo_item_spam_count} onChange={(v) => set({ combo_item_spam_count: v })} />
            <NumberInput label="Item Delay" value={config.combo_item_delay_ms} onChange={(v) => set({ combo_item_delay_ms: v })} suffix="ms" />
            <NumberInput label="Earthbind Presses" value={config.earthbind_press_count} onChange={(v) => set({ earthbind_press_count: v })} />
            <NumberInput label="Earthbind Interval" value={config.earthbind_press_interval_ms} onChange={(v) => set({ earthbind_press_interval_ms: v })} suffix="ms" />
            <NumberInput label="Poof Presses" value={config.poof_press_count} onChange={(v) => set({ poof_press_count: v })} />
            <NumberInput label="Poof Interval" value={config.poof_press_interval_ms} onChange={(v) => set({ poof_press_interval_ms: v })} suffix="ms" />
          </div>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Danger Abilities">
          <Toggle label="Auto-Dig on Danger" checked={config.auto_dig_on_danger} onChange={(v) => set({ auto_dig_on_danger: v })} />
          <Slider label="Dig HP Threshold" value={config.dig_hp_threshold_percent} min={10} max={80} onChange={(v) => set({ dig_hp_threshold_percent: v })} suffix="%" />
          <Toggle label="Auto-MegaMeepo on Danger" checked={config.auto_megameepo_on_danger} onChange={(v) => set({ auto_megameepo_on_danger: v })} />
          <Slider label="MegaMeepo HP Threshold" value={config.megameepo_hp_threshold_percent} min={10} max={80} onChange={(v) => set({ megameepo_hp_threshold_percent: v })} suffix="%" />
          <NumberInput label="Defensive Cooldown" value={config.defensive_trigger_cooldown_ms} onChange={(v) => set({ defensive_trigger_cooldown_ms: v })} suffix="ms" />
        </Card>

        <Card title="Farm Assist" collapsible>
          <Toggle label="Enabled" checked={config.farm_assist.enabled} onChange={(v) => setFarm({ enabled: v })} />
          <KeyInput label="Toggle Key" value={config.farm_assist.toggle_key} onChange={(v) => setFarm({ toggle_key: v })} />
          <NumberInput label="Pulse Interval" value={config.farm_assist.pulse_interval_ms} onChange={(v) => setFarm({ pulse_interval_ms: v })} suffix="ms" />
          <Slider label="Min Mana" value={config.farm_assist.minimum_mana_percent} min={0} max={100} onChange={(v) => setFarm({ minimum_mana_percent: v })} suffix="%" />
          <Slider label="Min Health" value={config.farm_assist.minimum_health_percent} min={0} max={100} onChange={(v) => setFarm({ minimum_health_percent: v })} suffix="%" />
          <Toggle label="Right-Click After Poof" checked={config.farm_assist.right_click_after_poof} onChange={(v) => setFarm({ right_click_after_poof: v })} />
          <Toggle label="Suspend on Danger" checked={config.farm_assist.suspend_on_danger} onChange={(v) => setFarm({ suspend_on_danger: v })} />
          <NumberInput label="Suspend After Combo" value={config.farm_assist.suspend_after_manual_combo_ms} onChange={(v) => setFarm({ suspend_after_manual_combo_ms: v })} suffix="ms" />
        </Card>
      </div>
    </>
  );
}
