import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { NumberInput } from "../../common/NumberInput";
import { KeyInput } from "../../common/KeyInput";
import { useConfigStore } from "../../../stores/configStore";

export default function EarthSpiritConfig() {
  const config = useConfigStore((s) => s.config.heroes.earth_spirit);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("earth_spirit", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Silence Combo">
          <Toggle label="Enable Both Combos" checked={config.enabled} onChange={(v) => set({ enabled: v })} />
          <Toggle label="Remap Grip Key" checked={config.silence_combo_enabled} onChange={(v) => set({ silence_combo_enabled: v })} />
          <KeyInput label="Stone Remnant Key" value={config.remnant_key} onChange={(v) => set({ remnant_key: v })} />
          <KeyInput label="Geomagnetic Grip Key" value={config.grip_key} onChange={(v) => set({ grip_key: v })} />
          <NumberInput label="Remnant Delay" value={config.silence_remnant_delay_ms} onChange={(v) => set({ silence_remnant_delay_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            Earth Spirit has no silence button. One press of the grip key drops
            a Stone Remnant at your cursor and then grips it back, and the
            silence lands on everything the remnant passes through on the way.
          </p>
          <p className="text-xs text-muted">
            <strong>Aim past the target, not at it.</strong> The remnant travels
            back toward Earth Spirit, so whoever is between your cursor and you
            is who gets silenced. That part stays manual — the combo only
            removes the second key press.
          </p>
          <p className="text-xs text-muted">
            Raise the remnant delay if the grip reaches for an older remnant, or
            for nothing at all — the new one has to exist server-side before the
            grip resolves.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Enhanced Roll">
          <Toggle label="Remap Roll Key" checked={config.roll_combo_enabled} onChange={(v) => set({ roll_combo_enabled: v })} />
          <KeyInput label="Rolling Boulder Key" value={config.roll_key} onChange={(v) => set({ roll_key: v })} />
          <Toggle label="Double-Tap Roll Key" checked={config.roll_double_tap} onChange={(v) => set({ roll_double_tap: v })} />
          <NumberInput label="Double-Tap Delay" value={config.roll_double_tap_delay_ms} onChange={(v) => set({ roll_double_tap_delay_ms: v })} suffix="ms" />
          <NumberInput label="Aim Window Before Remnant" value={config.roll_to_remnant_delay_ms} onChange={(v) => set({ roll_to_remnant_delay_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            A roll that passes through a remnant travels 1600 units instead of
            800 and moves much faster, so the good roll is always the two-key
            one.
          </p>
          <p className="text-xs text-muted">
            <strong>This one rolls first, then places the remnant</strong> — the
            opposite order to the silence above. Rolling Boulder has a ~600ms
            windup before Earth Spirit starts moving, and a remnant dropped into
            the path during that window still counts. Casting the roll first
            locks in the direction, so the windup is yours to move the cursor
            and put the remnant exactly where the boulder will pass.
          </p>
          <p className="text-xs text-muted">
            <strong>Aim window</strong> is how long you get between the roll
            firing and the remnant landing at your cursor. It is a human-sized
            delay, not a server-timing one. Raise it if you cannot move the
            cursor in time — but keep it plus the double-tap delay under ~600ms,
            or the boulder is already rolling and a remnant placed behind it
            does nothing, which looks exactly like the combo firing with no
            effect.
          </p>
          <p className="text-xs text-muted">
            <strong>Double-tap is the setting to try both ways.</strong> Rolling
            Boulder is the one ability commonly left off quickcast, where the
            first press only arms the cursor and a second press fires it. Leave
            it on for a normal-cast roll key; turn it off if you enable
            quickcast for the roll, or if the second tap cancels the targeting
            instead of casting. With it off the combo sends a single press, and
            the aim window is measured from that press instead.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Safety">
          <Toggle label="Only Remap Grip When Ready" checked={config.require_grip_ready} onChange={(v) => set({ require_grip_ready: v })} />
          <Toggle label="Only Remap Roll When Ready" checked={config.require_roll_ready} onChange={(v) => set({ require_roll_ready: v })} />
          <p className="text-xs text-muted">
            Passes the key straight through when its ability is unlevelled or on
            cooldown, so a wasted press never spends a remnant charge on a spell
            that cannot fire.
          </p>
          <p className="text-xs text-muted">
            Neither check reads Stone Remnant itself. It is charge-based, and
            GSI reports charges unreliably — gating on it would leave the combo
            dead while remnants are visibly banked.
          </p>
          <p className="text-xs text-muted">
            <strong>Quickcast must be on for Stone Remnant and the grip.</strong>{" "}
            Both are point-target: without quickcast the first press only arms
            the cursor and the second cancels the targeting instead of
            resolving it.
          </p>
        </Card>
      </div>
    </>
  );
}
