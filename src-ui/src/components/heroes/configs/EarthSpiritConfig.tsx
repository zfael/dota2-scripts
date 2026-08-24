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
          <NumberInput label="Delay Before Remnant" value={config.roll_to_remnant_delay_ms} onChange={(v) => set({ roll_to_remnant_delay_ms: v })} suffix="ms" />
          <Toggle label="Hold ALT For Remnant (Self-Cast)" checked={config.roll_remnant_alt} onChange={(v) => set({ roll_remnant_alt: v })} />
          <Toggle label="Double-Tap Remnant Key (Self-Cast)" checked={config.roll_remnant_double_tap} onChange={(v) => set({ roll_remnant_double_tap: v })} />
          <NumberInput label="Remnant Double-Tap Delay" value={config.roll_remnant_double_tap_delay_ms} onChange={(v) => set({ roll_remnant_double_tap_delay_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            A roll that passes through a remnant travels 1600 units instead of
            800 and moves much faster, so the good roll is always the two-key
            one.
          </p>
          <p className="text-xs text-muted">
            <strong>This one rolls first, then self-casts the remnant</strong> —
            the opposite order to the silence above. Rolling Boulder has a
            ~600ms windup before Earth Spirit starts moving, and a remnant
            dropped into the path during that window still counts.
          </p>
          <p className="text-xs text-muted">
            <strong>Self-cast is what removes the aiming.</strong> It drops the
            stone on Earth Spirit himself, and the roll starts from Earth
            Spirit — so the boulder passes through it every time, wherever your
            cursor happens to be. Cast the roll to pick the direction; the stone
            takes care of itself.
          </p>
          <p className="text-xs text-muted">
            Two routes to self-cast, because which one works depends on your
            Dota settings, and both ship on. <strong>ALT</strong> is Dota's
            self-cast modifier and is the route that still works with quickcast
            on the remnant key — with quickcast, a plain double-tap just places
            two stones at the cursor. If your Dota alt-pings abilities instead,
            this will ping Stone Remnant to your team; turn it off and rely on
            the double-tap.
          </p>
          <p className="text-xs text-muted">
            Turning <em>both</em> self-cast options off puts the remnant back at
            your cursor, which then needs real aiming time — raise the delay
            above to ~300ms if you do that.
          </p>
          <p className="text-xs text-muted">
            <strong>The roll's own double-tap is separate.</strong> Rolling
            Boulder is commonly left off quickcast, where the first press only
            arms the cursor and a second press fires it. Turn it off if you
            enable quickcast for the roll, or if the second tap cancels the
            targeting instead of casting.
          </p>
          <p className="text-xs text-muted">
            Keep all the delays in this card summing to well under ~600ms. Past
            that the boulder is already rolling and a remnant placed behind it
            does nothing, which looks exactly like the combo firing with no
            effect.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Scepter Escape">
          <Toggle label="Self-Petrify When In Danger" checked={config.auto_petrify_on_danger} onChange={(v) => set({ auto_petrify_on_danger: v })} />
          <KeyInput label="Enchant Remnant Key" value={config.petrify_key} onChange={(v) => set({ petrify_key: v })} />
          <NumberInput label="HP Threshold" value={config.petrify_hp_threshold_percent} onChange={(v) => set({ petrify_hp_threshold_percent: v })} suffix="%" />
          <NumberInput label="Retry Cooldown" value={config.petrify_trigger_cooldown_ms} onChange={(v) => set({ petrify_trigger_cooldown_ms: v })} suffix="ms" />
          <Toggle label="Hold ALT For Enchant Remnant (Self-Cast)" checked={config.petrify_alt} onChange={(v) => set({ petrify_alt: v })} />
          <Toggle label="Double-Tap Enchant Remnant Key (Self-Cast)" checked={config.petrify_double_tap} onChange={(v) => set({ petrify_double_tap: v })} />
          <NumberInput label="Enchant Remnant Double-Tap Delay" value={config.petrify_double_tap_delay_ms} onChange={(v) => set({ petrify_double_tap_delay_ms: v })} suffix="ms" />
          <Toggle label="Kick Yourself Out With Boulder Smash" checked={config.petrify_smash_enabled} onChange={(v) => set({ petrify_smash_enabled: v })} />
          <KeyInput label="Boulder Smash Key" value={config.smash_key} onChange={(v) => set({ smash_key: v })} />
          <NumberInput label="Delay Before Smash" value={config.petrify_to_smash_delay_ms} onChange={(v) => set({ petrify_to_smash_delay_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            With an Aghanim's Scepter you get Enchant Remnant.{" "}
            <strong>Self-cast, it turns you into a Stone Remnant</strong> —
            untargetable, which is the save — and a remnant is a legal Boulder
            Smash target, so the kick that follows launches you out of whatever
            was killing you.
          </p>
          <p className="text-xs text-muted">
            This one fires by itself, off GSI, when the danger detector trips
            and your HP is at or below the threshold. It is <em>not</em> gated
            by the combo toggles above — turning the key remaps off leaves the
            panic button armed.
          </p>
          <p className="text-xs text-muted">
            Keep the HP threshold well under your danger-detection threshold.
            The escape takes you out of the fight entirely, so it should be the
            last thing tried, after the defensive items have gone off.
          </p>
          <p className="text-xs text-muted">
            <strong>Self-cast is what makes the remnant be you.</strong> Same
            two routes as the roll above, both on:{" "}
            <strong>ALT</strong> survives quickcast, the double-tap is Dota's
            default binding. Turning both off does not disable the escape — it
            petrifies whoever your cursor was over, which is a completely
            different spell.
          </p>
          <p className="text-xs text-muted">
            The smash is a single plain press — not self-cast, not aimed. Its
            delay waits on the petrify <em>resolving</em>, not just on the key
            clearing: there is no remnant to kick until you are actually stone.
          </p>
          <p className="text-xs text-muted">
            The retry cooldown is longer than the other auto-casts on purpose.
            You stay petrified for several seconds, and GSI keeps reporting the
            low HP that triggered it the whole time.
          </p>
          <p className="text-xs text-muted">
            Without a scepter, Enchant Remnant is not in the GSI payload at all
            and none of this ever runs.
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
