import { Link } from "react-router-dom";
import { Alert } from "../components/common/Alert";
import { Card } from "../components/common/Card";
import { NumberInput } from "../components/common/NumberInput";
import { SettingRow } from "../components/common/SettingRow";
import { useConfigStore } from "../stores/configStore";

export default function Boots() {
  const phaseBoots = useConfigStore((s) => s.config.phase_boots_automation);
  const updatePhaseBoots = (updates: Partial<typeof phaseBoots>) =>
    useConfigStore.getState().updateConfig("phase_boots_automation", updates);

  return (
    <div className="max-w-[620px] p-6">
      <Card title="Phase Boots">
        <SettingRow
          label="Enable Phase Boots Automation"
          checked={phaseBoots.enabled}
          onChange={(v) => updatePhaseBoots({ enabled: v })}
        />
        <NumberInput
          label="Minimum Movement Distance"
          value={phaseBoots.minimum_distance_units}
          onChange={(v) => updatePhaseBoots({ minimum_distance_units: v })}
          suffix="u"
          hint="Only triggers once the hero has actually walked at least this far during the current movement segment."
        />
        <Alert tone="info" title="Breaks invisibility">
          Activating Phase Boots breaks Shadow Blade and Silver Edge invisibility.
          Holding it — along with every other automation that would do the same —
          is a single switch on{" "}
          <Link to="/survivability" className="text-accent-text hover:underline">
            Survivability
          </Link>
          .
        </Alert>
      </Card>
    </div>
  );
}
