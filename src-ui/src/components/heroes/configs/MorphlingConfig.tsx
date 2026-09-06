import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { NumberInput } from "../../common/NumberInput";
import { KeyInput } from "../../common/KeyInput";
import { useConfigStore } from "../../../stores/configStore";

/** HP one attribute point is worth on the current patch, measured from a live
 *  capture. Used only to show the target in points alongside the HP figure the
 *  automation actually works in — nothing here depends on it being right. */
const HP_PER_ATTRIBUTE_POINT = 22;

export default function MorphlingConfig() {
  const config = useConfigStore((s) => s.config.heroes.morphling);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("morphling", updates);

  const points = Math.round(config.target_hp_gain / HP_PER_ATTRIBUTE_POINT);

  return (
    <>
      <div className="space-y-4">
        <Card title="Attribute Shift On Danger">
          <Toggle label="Shift To Strength When In Danger" checked={config.enabled} onChange={(v) => set({ enabled: v })} />
          <KeyInput label="Strength Gain Key" value={config.strength_key} onChange={(v) => set({ strength_key: v })} />
          <KeyInput label="Agility Gain Key" value={config.agility_key} onChange={(v) => set({ agility_key: v })} />
          <p className="text-xs text-muted">
            These are pressed as ordinary keys, so they have to match your
            in-game bindings — Dota's keybinds cannot be read.
          </p>
        </Card>

        <Card title="How Much A Fight Buys">
          <NumberInput label="Target HP Gain" value={config.target_hp_gain} onChange={(v) => set({ target_hp_gain: v })} suffix="HP" />
          <p className="text-xs text-muted">
            Roughly <strong>{points} strength</strong> at {HP_PER_ATTRIBUTE_POINT} HP
            per point, about {(config.target_hp_gain / 345).toFixed(1)}s of
            shifting. Raise it if you are still dying through the shift.
          </p>
          <p className="text-xs text-muted">
            Set in HP rather than attribute points on purpose: max HP is the only
            thing GSI reports about a shift, and what a point is worth changes
            between patches.
          </p>
          <p className="text-xs text-muted">
            A fight that keeps going does not keep buying — the target is fixed
            when it starts, so one long teamfight cannot ratchet into a full
            shift. A <em>new</em> fight always buys another slice, including one
            that arrives before the shift back was due.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Shifting Back">
          <Toggle label="Return To Agility Automatically" checked={config.return_to_agility} onChange={(v) => set({ return_to_agility: v })} />
          <NumberInput label="Quiet Period First" value={config.return_delay_seconds} onChange={(v) => set({ return_delay_seconds: v })} suffix="s" />
          <NumberInput label="Health Floor" value={config.return_min_health_percent} onChange={(v) => set({ return_min_health_percent: v })} suffix="%" />
          <p className="text-xs text-muted">
            The quiet period is separate from the danger detector's own clear
            delay on purpose: not taking damage for a moment is not the same as
            the fight being over. Raise it if the shift back keeps happening
            while you are still in one.
          </p>
          <p className="text-xs text-muted">
            Losing max HP costs current HP, so the health floor is what stops a
            shift back from finishing what the enemy started. Danger returning
            mid-shift reverses it immediately.
          </p>
          <p className="text-xs text-muted">
            Turn the toggle off to keep the shift back for yourself — strength on
            danger still fires.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Timing">
          <NumberInput label="Shift Cap" value={config.max_shift_seconds} onChange={(v) => set({ max_shift_seconds: v })} suffix="s" />
          <NumberInput label="Finished After" value={config.plateau_ms} onChange={(v) => set({ plateau_ms: v })} suffix="ms" />
          <NumberInput label="Press Settle" value={config.press_settle_ms} onChange={(v) => set({ press_settle_ms: v })} suffix="ms" />
          <NumberInput label="Manual Override" value={config.manual_override_seconds} onChange={(v) => set({ manual_override_seconds: v })} suffix="s" />
          <p className="text-xs text-muted">
            "Finished after" is how long max HP must stand still before the shift
            counts as over — either stopped, or out of pool.
          </p>
          <p className="text-xs text-muted">
            <strong>Leave press settle alone</strong> unless you have re-measured
            it. A press takes 95–391ms to land, and a second press inside that
            window <em>stops</em> the shift instead of adjusting it.
          </p>
          <p className="text-xs text-muted">
            Shifting by hand stands the automation down for the manual override
            window, and keeps it down for as long as your shift is still moving.
            There is no keyboard hook — it works this out from max HP, which takes
            about half a second.
          </p>
        </Card>
      </div>
    </>
  );
}
