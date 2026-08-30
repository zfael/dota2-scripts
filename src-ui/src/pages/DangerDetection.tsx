import { Link } from "react-router-dom";
import { Card } from "../components/common/Card";
import { Slider } from "../components/common/Slider";
import { SettingRow } from "../components/common/SettingRow";
import { useConfigStore } from "../stores/configStore";

export default function DangerDetection() {
  const danger = useConfigStore((s) => s.config.danger_detection);
  const updateDanger = (updates: Partial<typeof danger>) =>
    useConfigStore.getState().updateConfig("danger_detection", updates);

  return (
    <div className="space-y-4 p-6">
      <p className="text-subtle">
        When the runtime decides you are in danger. What it then does about it is
        configured on{" "}
        <Link to="/survivability" className="text-accent-text hover:underline">
          Survivability
        </Link>
        .
      </p>

      <div className="grid grid-cols-1 items-start gap-4 lg:grid-cols-2">
        <Card title="Core Settings">
          <SettingRow
            label="Enable Danger Detection"
            checked={danger.enabled}
            onChange={(v) => updateDanger({ enabled: v })}
          />
          <Slider
            label="HP Threshold"
            value={danger.hp_threshold_percent}
            min={30}
            max={90}
            onChange={(v) => updateDanger({ hp_threshold_percent: v })}
            suffix="%"
          />
          <Slider
            label="Rapid Loss Threshold"
            value={danger.rapid_loss_hp}
            min={50}
            max={300}
            onChange={(v) => updateDanger({ rapid_loss_hp: v })}
            suffix=" HP"
          />
          <Slider
            label="Burst Time Window"
            value={danger.time_window_ms}
            min={100}
            max={2000}
            onChange={(v) => updateDanger({ time_window_ms: v })}
            suffix=" ms"
          />
          <Slider
            label="Clear Delay"
            value={danger.clear_delay_seconds}
            min={1}
            max={10}
            onChange={(v) => updateDanger({ clear_delay_seconds: v })}
            suffix=" s"
          />
        </Card>

        <Card
          title="Responses"
          subtitle="Healing thresholds, defensive items, dispels and neutral items all react to the state configured here"
          flushBody
        >
          <Link
            to="/survivability"
            className="flex items-center justify-between rounded-md border border-border bg-elevated p-3 text-sm text-content transition-colors hover:bg-raised"
          >
            <span>Survivability</span>
            <span className="text-accent-text">Configure →</span>
          </Link>
        </Card>
      </div>
    </div>
  );
}
