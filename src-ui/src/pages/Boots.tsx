import { Link } from "react-router-dom";
import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { NumberInput } from "../components/common/NumberInput";
import { useConfigStore } from "../stores/configStore";

export default function Boots() {
  const phaseBoots = useConfigStore((s) => s.config.phase_boots_automation);
  const updatePhaseBoots = (updates: Partial<typeof phaseBoots>) =>
    useConfigStore.getState().updateConfig("phase_boots_automation", updates);
  const treads = useConfigStore((s) => s.config.power_treads);
  const updateTreads = (updates: Partial<typeof treads>) =>
    useConfigStore.getState().updateConfig("power_treads", updates);

  return (
    <div className="space-y-6 p-6">
      <h2 className="text-xl font-semibold">Boots</h2>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="Phase Boots">
            <Toggle
              label="Enable Phase Boots Automation"
              checked={phaseBoots.enabled}
              onChange={(v) => updatePhaseBoots({ enabled: v })}
            />
            <NumberInput
              label="Minimum Movement Distance"
              value={phaseBoots.minimum_distance_units}
              onChange={(v) => updatePhaseBoots({ minimum_distance_units: v })}
              suffix="u"
            />
            <p className="text-xs text-subtle">
              Only triggers once the hero has actually walked at least this far
              during the current movement segment.
            </p>
            <p className="text-xs text-subtle">
              Activating Phase Boots breaks Shadow Blade and Silver Edge
              invisibility. Holding it — along with every other automation that
              would do the same — is now a single switch on{" "}
              <Link to="/survivability" className="text-gold hover:underline">
                Survivability
              </Link>
              .
            </p>
          </Card>
        </div>

        <div className="space-y-4">
          <Card title="Power Treads">
            <Toggle
              label="Enable Power Treads Switching"
              checked={treads.enabled}
              onChange={(v) => updateTreads({ enabled: v })}
            />
            <p className="text-xs text-subtle">
              Experimental. GSI never says which attribute the treads are on,
              so it is read from how max HP and max mana move. The state is
              tracked even while this is off.
            </p>
            <Toggle
              label="Switch for Mana Spells and Items"
              checked={treads.cast_switch}
              disabled={!treads.enabled}
              onChange={(v) => updateTreads({ cast_switch: v })}
            />
            <Toggle
              label="Hold STR in Danger"
              checked={treads.danger_switch}
              disabled={!treads.enabled}
              onChange={(v) => updateTreads({ danger_switch: v })}
            />
            <Toggle
              label="Heal in the Smallest Pool"
              checked={treads.heal_switch}
              disabled={!treads.enabled}
              onChange={(v) => updateTreads({ heal_switch: v })}
            />
            <Toggle
              label="Verify State When Idle"
              checked={treads.auto_verify}
              disabled={!treads.enabled}
              onChange={(v) => updateTreads({ auto_verify: v })}
            />
            <Toggle
              label="My Switches Set Home"
              checked={treads.manual_switch_sets_home}
              disabled={!treads.enabled}
              onChange={(v) => updateTreads({ manual_switch_sets_home: v })}
            />
            <NumberInput
              label="Don't Leave STR to Cast Below"
              value={treads.min_health_percent}
              min={0}
              max={100}
              disabled={!treads.enabled}
              onChange={(v) => updateTreads({ min_health_percent: v })}
              suffix="% HP"
            />
            <p className="text-xs text-subtle">
              Home is your hero's primary attribute (STR for universal heroes)
              until you switch yourself. Verifying cycles the treads once when
              you are idle and safe. Timings are in config.toml.
            </p>
          </Card>
        </div>
      </div>
    </div>
  );
}
