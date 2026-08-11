import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { NumberInput } from "../../common/NumberInput";
import { KeyInput } from "../../common/KeyInput";
import { useConfigStore } from "../../../stores/configStore";

export default function SlarkConfig() {
  const config = useConfigStore((s) => s.config.heroes.slark);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("slark", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Directional Pounce">
          <Toggle label="Enable Pounce Intercept" checked={config.enabled} onChange={(v) => set({ enabled: v })} />
          <KeyInput label="Pounce Key" value={config.pounce_key} onChange={(v) => set({ pounce_key: v })} />
          <NumberInput label="Turn Delay" value={config.turn_delay_ms} onChange={(v) => set({ turn_delay_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            Faces the cursor before casting, so Pounce leaps where you are
            pointing instead of wherever Slark happened to be facing. Raise the
            turn delay if the leap fires before Slark finishes turning.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Auto Dark Pact">
          <Toggle label="Cleanse Debuffs With Dark Pact" checked={config.auto_dark_pact_on_debuff} onChange={(v) => set({ auto_dark_pact_on_debuff: v })} />
          <KeyInput label="Dark Pact Key" value={config.dark_pact_key} onChange={(v) => set({ dark_pact_key: v })} />
          <NumberInput label="Settle Window" value={config.dark_pact_delay_ms} onChange={(v) => set({ dark_pact_delay_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            Casts Dark Pact whenever GSI reports a debuff. GSI never says
            <em> which</em> debuff landed, so this fires on trivial ones too —
            turn it off if you would rather keep Dark Pact for farming. The
            settle window waits out a burst so one cast cleanses all of it.
          </p>
        </Card>

        <Card title="Safety">
          <Toggle label="Only Intercept When Ready" checked={config.require_ability_ready} onChange={(v) => set({ require_ability_ready: v })} />
          <p className="text-xs text-muted">
            Passes the key straight through when Pounce is unlevelled or on
            cooldown, so a wasted press never turns Slark toward the cursor and
            walks him into the enemy.
          </p>
        </Card>
      </div>
    </>
  );
}
