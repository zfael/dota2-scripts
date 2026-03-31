import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { NumberInput } from "../../common/NumberInput";
import { useConfigStore } from "../../../stores/configStore";

export default function ShadowFiendConfig() {
  const config = useConfigStore((s) => s.config.heroes.shadow_fiend);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("shadow_fiend", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Raze Intercept">
          <Toggle label="Enable Raze Intercept" checked={config.raze_intercept_enabled} onChange={(v) => set({ raze_intercept_enabled: v })} />
          <NumberInput label="Raze Delay" value={config.raze_delay_ms} onChange={(v) => set({ raze_delay_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            Intercepts Q/W/E to face cursor direction before razing.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Ultimate Intercept">
          <Toggle label="Auto-BKB on Ultimate" checked={config.auto_bkb_on_ultimate} onChange={(v) => set({ auto_bkb_on_ultimate: v })} />
          <Toggle label="Auto-D Ability on Ultimate" checked={config.auto_d_on_ultimate} onChange={(v) => set({ auto_d_on_ultimate: v })} />
        </Card>
      </div>
    </>
  );
}

