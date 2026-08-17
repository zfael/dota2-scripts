import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { NumberInput } from "../../common/NumberInput";
import { KeyInput } from "../../common/KeyInput";
import { useConfigStore } from "../../../stores/configStore";

export default function EmberSpiritConfig() {
  const config = useConfigStore((s) => s.config.heroes.ember_spirit);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("ember_spirit", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Remnant Chase">
          <Toggle label="Enable Remnant Chase" checked={config.enabled} onChange={(v) => set({ enabled: v })} />
          <KeyInput label="Fire Remnant Key" value={config.remnant_key} onChange={(v) => set({ remnant_key: v })} />
          <KeyInput label="Activate Remnant Key" value={config.activate_key} onChange={(v) => set({ activate_key: v })} />
          <NumberInput label="Activate Delay" value={config.activate_delay_ms} onChange={(v) => set({ activate_delay_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            One press of the combo trigger key drops a Fire Remnant and then
            dashes to it, so chasing costs one key instead of two.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Auto Flame Guard">
          <Toggle label="Flame Guard On Danger" checked={config.auto_flame_guard_on_danger} onChange={(v) => set({ auto_flame_guard_on_danger: v })} />
          <KeyInput label="Flame Guard Key" value={config.flame_guard_key} onChange={(v) => set({ flame_guard_key: v })} />
          <NumberInput label="HP Threshold" value={config.flame_guard_hp_threshold_percent} onChange={(v) => set({ flame_guard_hp_threshold_percent: v })} suffix="%" />
          <NumberInput label="Retry Cooldown" value={config.flame_guard_trigger_cooldown_ms} onChange={(v) => set({ flame_guard_trigger_cooldown_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            Casts Flame Guard when the danger detector trips and HP is at or
            below the threshold. Both conditions must hold — danger alone fires
            on any rapid HP drop, including at full health.
          </p>
          <p className="text-xs text-muted">
            The threshold is deliberately higher than the other danger
            thresholds: Flame Guard is a damage shield, so it is worth most when
            it goes up before the burst lands, not after.
          </p>
          <p className="text-xs text-muted">
            Independent of the remnant chase toggle above — turning the chase
            off leaves this running.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Before You Use It">
          <p className="text-xs text-muted">
            Fired by the global combo trigger key set under Settings, not by a
            key of its own, and only while Ember Spirit is the active hero.
          </p>
          <p className="text-xs text-muted">
            <strong>Quickcast must be on for Fire Remnant.</strong> Without it
            the first press only arms the cursor, and the activate that follows
            cancels the targeting instead of placing the remnant.
          </p>
          <p className="text-xs text-muted">
            Raise the activate delay if the dash skips the remnant you just
            placed — it has to exist server-side before the activate can pick it
            up. Lower it if the chase feels sluggish.
          </p>
        </Card>
      </div>
    </>
  );
}
