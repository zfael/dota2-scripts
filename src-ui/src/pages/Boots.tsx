import { Link } from "react-router-dom";
import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { NumberInput } from "../components/common/NumberInput";
import { useConfigStore } from "../stores/configStore";

export default function Boots() {
  const phaseBoots = useConfigStore((s) => s.config.phase_boots_automation);
  const updatePhaseBoots = (updates: Partial<typeof phaseBoots>) =>
    useConfigStore.getState().updateConfig("phase_boots_automation", updates);

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
      </div>
    </div>
  );
}
