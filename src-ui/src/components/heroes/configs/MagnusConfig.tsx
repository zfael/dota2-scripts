import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { NumberInput } from "../../common/NumberInput";
import { KeyInput } from "../../common/KeyInput";
import { useConfigStore } from "../../../stores/configStore";

export default function MagnusConfig() {
  const config = useConfigStore((s) => s.config.heroes.magnus);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("magnus", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Directional Reverse Polarity">
          <Toggle label="Enable Ultimate Intercept" checked={config.enabled} onChange={(v) => set({ enabled: v })} />
          <KeyInput label="Ultimate Key" value={config.ultimate_key} onChange={(v) => set({ ultimate_key: v })} />
          <NumberInput label="Turn Delay" value={config.turn_delay_ms} onChange={(v) => set({ turn_delay_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            Faces the cursor before casting, so Reverse Polarity drags enemies to
            the arc your Skewer is aimed at. Raise the turn delay if the ultimate
            fires before Magnus finishes turning.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Safety">
          <Toggle label="Only Intercept When Ready" checked={config.require_ability_ready} onChange={(v) => set({ require_ability_ready: v })} />
          <p className="text-xs text-muted">
            Passes the key straight through when Reverse Polarity is unlevelled
            or on cooldown, so a wasted press never turns Magnus toward the
            cursor.
          </p>
        </Card>

        <Card title="Camera Recentre">
          <Toggle label="Centre Camera After Ultimate" checked={config.center_camera_on_ultimate} onChange={(v) => set({ center_camera_on_ultimate: v })} />
          <KeyInput label="Hero Select Key" value={config.camera_center_key} onChange={(v) => set({ camera_center_key: v })} />
          <NumberInput label="Camera Delay" value={config.camera_center_delay_ms} onChange={(v) => set({ camera_center_delay_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            Double-taps your hero-select key after the cast so the view snaps
            back to Magnus and Skewer is easy to aim. Must match your in-game
            binding — accepts a character like <code>1</code> or a named key
            like <code>F1</code>.
          </p>
        </Card>
      </div>
    </>
  );
}
