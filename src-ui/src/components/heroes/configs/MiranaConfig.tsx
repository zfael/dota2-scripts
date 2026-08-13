import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { NumberInput } from "../../common/NumberInput";
import { KeyInput } from "../../common/KeyInput";
import { useConfigStore } from "../../../stores/configStore";

export default function MiranaConfig() {
  const config = useConfigStore((s) => s.config.heroes.mirana);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("mirana", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Directional Leap">
          <Toggle label="Enable Leap Intercept" checked={config.enabled} onChange={(v) => set({ enabled: v })} />
          <KeyInput label="Leap Key" value={config.leap_key} onChange={(v) => set({ leap_key: v })} />
          <NumberInput label="Turn Delay" value={config.turn_delay_ms} onChange={(v) => set({ turn_delay_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            Faces the cursor before casting, so Leap jumps where you are pointing
            instead of wherever Mirana happened to be facing. Raise the turn
            delay if the leap fires before she finishes turning.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Safety">
          <Toggle label="Only Intercept When Ready" checked={config.require_ability_ready} onChange={(v) => set({ require_ability_ready: v })} />
          <p className="text-xs text-muted">
            Passes the key straight through when Leap is unlevelled or on
            cooldown, so a wasted press never turns Mirana toward the cursor and
            walks her into the enemy.
          </p>
          <p className="text-xs text-muted">
            Leap is charge-based. If you ever see the intercept skipped while
            Leap is visibly castable, turn this off — the readiness check reads
            what GSI reports, and a banked charge may not read as castable.
          </p>
        </Card>
      </div>
    </>
  );
}
