import { Link } from "react-router-dom";
import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { Slider } from "../components/common/Slider";
import { useConfigStore } from "../stores/configStore";

export default function DangerDetection() {
  const danger = useConfigStore((s) => s.config.danger_detection);
  const updateDanger = (updates: Partial<typeof danger>) =>
    useConfigStore.getState().updateConfig("danger_detection", updates);

  return (
    <div className="space-y-6 p-6">
      <div className="space-y-1">
        <h2 className="text-xl font-semibold">Danger Detection</h2>
        <p className="text-xs text-subtle">
          When the runtime decides you are in danger. What it then does about it
          is configured on{" "}
          <Link to="/survivability" className="text-gold hover:underline">
            Survivability
          </Link>
          .
        </p>
      </div>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="Core Settings">
            <Toggle label="Enable Danger Detection" checked={danger.enabled} onChange={(v) => updateDanger({ enabled: v })} />
            <Slider label="HP Threshold" value={danger.hp_threshold_percent} min={30} max={90} onChange={(v) => updateDanger({ hp_threshold_percent: v })} suffix="%" />
            <Slider label="Rapid Loss Threshold" value={danger.rapid_loss_hp} min={50} max={300} onChange={(v) => updateDanger({ rapid_loss_hp: v })} suffix=" HP" />
            <Slider label="Burst Time Window" value={danger.time_window_ms} min={100} max={2000} onChange={(v) => updateDanger({ time_window_ms: v })} suffix="ms" />
            <Slider label="Clear Delay" value={danger.clear_delay_seconds} min={1} max={10} onChange={(v) => updateDanger({ clear_delay_seconds: v })} suffix="s" />
          </Card>
        </div>

        <div className="space-y-4">
          <Card title="Responses">
            <p className="text-xs text-subtle">
              Healing thresholds, defensive items, dispels and neutral items all
              react to the state configured here.
            </p>
            <Link
              to="/survivability"
              className="flex items-center justify-between rounded-md border border-border bg-base p-3 transition-colors hover:bg-elevated"
            >
              <span className="text-sm text-content">Survivability</span>
              <span className="text-xs text-gold">Configure →</span>
            </Link>
          </Card>
        </div>
      </div>
    </div>
  );
}
