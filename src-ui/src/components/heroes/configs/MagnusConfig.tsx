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

        <Card title="Saved Point">
          <KeyInput label="Save Point Key" value={config.capture_anchor_key} onChange={(v) => set({ capture_anchor_key: v })} />
          <p className="text-xs text-muted">
            Status:{" "}
            {config.anchor_set ? (
              <span className="text-terminal">
                saved at {(config.anchor_x_fraction * 100).toFixed(1)}%,{" "}
                {(config.anchor_y_fraction * 100).toFixed(1)}% of Dota's window
              </span>
            ) : (
              <span className="text-warning">not saved yet</span>
            )}
          </p>
          <p className="text-xs text-muted">
            In game, hover a spot on the minimap (your base, say) and press{" "}
            <code>{config.capture_anchor_key}</code>. Press it again anywhere to
            move the point. The key only works while playing Magnus with Skewer
            To Saved Point on, and never reaches Dota then.
          </p>
          <Toggle label="Skewer To Saved Point" checked={config.skewer_to_anchor_enabled} onChange={(v) => set({ skewer_to_anchor_enabled: v })} />
          <KeyInput label="Skewer Key" value={config.skewer_key} onChange={(v) => set({ skewer_key: v })} />
          <p className="text-xs text-warning">
            Requires Quick Cast on the Skewer key in Dota's settings. With normal
            cast the press only arms targeting, and the cursor is already back
            before you can click.
          </p>
          <p className="text-xs text-muted">
            Moves the cursor to the saved point, presses Skewer there, and puts
            the cursor back. A point past Skewer's range gives a full-length
            Skewer toward it. Until a point is saved, Skewer casts normally.
          </p>
          <Toggle label="Only After Blink" checked={config.skewer_only_after_blink} onChange={(v) => set({ skewer_only_after_blink: v })} />
          <NumberInput label="Blink Press Window" value={config.blink_press_window_ms} onChange={(v) => set({ blink_press_window_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            For the Blink-in, Skewer-out play: Skewer goes to the saved point
            only when you carry a Blink and it is on cooldown, or you pressed its
            key within the window above (GSI reports the cooldown a moment
            late). Otherwise Skewer casts normally at your cursor. Off, every
            Skewer goes to the point.
          </p>
          <NumberInput label="Cursor Settle" value={config.anchor_settle_ms} onChange={(v) => set({ anchor_settle_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            How long the cursor rests on the point before and after the press.
            Raise it if the cast lands at your cursor instead of the point.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Safety">
          <Toggle label="Only Intercept When Ready" checked={config.require_ability_ready} onChange={(v) => set({ require_ability_ready: v })} />
          <p className="text-xs text-muted">
            Passes the key straight through when Reverse Polarity (or Skewer,
            for the saved-point option) is unlevelled or on cooldown, so a
            wasted press never turns Magnus or moves the cursor.
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
