import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { NumberInput } from "../../common/NumberInput";
import { KeyInput } from "../../common/KeyInput";
import { useConfigStore } from "../../../stores/configStore";

export default function SnapfireConfig() {
  const config = useConfigStore((s) => s.config.heroes.snapfire);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("snapfire", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Directional Cookie">
          <Toggle label="Enable Cookie Intercept" checked={config.enabled} onChange={(v) => set({ enabled: v })} />
          <KeyInput label="Trigger Key" value={config.trigger_key} onChange={(v) => set({ trigger_key: v })} />
          <NumberInput label="Turn Delay" value={config.turn_delay_ms} onChange={(v) => set({ turn_delay_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            Faces the cursor, then self-casts Firesnap Cookie so Snapfire leaps
            that way. Raise the turn delay if the leap fires before she finishes
            turning.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Ability Key">
          <KeyInput label="Firesnap Cookie" value={config.cookie_key} onChange={(v) => set({ cookie_key: v })} />
          <p className="text-xs text-muted">
            Must match your in-game binding. This key is never intercepted on its
            own, so cookie-ing an ally still works normally — the combo only
            self-casts it via ALT.
          </p>
        </Card>
      </div>
    </>
  );
}
