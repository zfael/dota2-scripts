import { Alert } from "../components/common/Alert";
import { Badge } from "../components/common/Badge";
import { Card } from "../components/common/Card";
import { MetricTable } from "../components/common/MetricTable";
import { useGameStore } from "../stores/gameStore";

function StatusTile({ active, label }: { active: boolean; label: string }) {
  return (
    <div className="flex items-center justify-between rounded-lg border border-border bg-surface p-4">
      <span className="text-sm text-content">{label}</span>
      <Badge tone={active ? "success" : "danger"} dot>
        {active ? "Active" : "Inactive"}
      </Badge>
    </div>
  );
}

export default function Diagnostics() {
  const diag = useGameStore((s) => s.diagnostics);

  return (
    <div className="space-y-4 p-6">
      <div className="grid grid-cols-3 gap-3">
        <StatusTile active={diag.gsiConnected} label="GSI Server" />
        <StatusTile active={diag.keyboardHookActive} label="Keyboard Hook" />
        <StatusTile active={diag.gsiConnected} label="Game State" />
      </div>

      <div className="grid grid-cols-1 items-start gap-4 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="GSI Pipeline" flushBody>
            <MetricTable
              rows={[
                { label: "Events Processed", value: diag.queueMetrics.eventsProcessed },
                { label: "Events Dropped", value: diag.queueMetrics.eventsDropped },
                { label: "Events Rejected", value: diag.queueMetrics.eventsRejected },
                {
                  label: "Queue Depth",
                  value: `${diag.queueMetrics.currentQueueDepth} / ${diag.queueMetrics.maxQueueDepth}`,
                },
              ]}
            />
            {diag.queueMetrics.eventsRejected > 0 && (
              <Alert tone="warning" className="mt-4">
                Dota sent payloads this build could not parse. The offending JSON is
                in <span className="font-mono">logs/gsi_rejected/</span>.
              </Alert>
            )}
          </Card>

          <Card title="Keyboard Hook" flushBody>
            <MetricTable
              rows={[
                { label: "Soul Ring State", value: diag.soulRingState },
                { label: "Blocked Keys", value: diag.blockedKeys.join(", ") || "None" },
              ]}
            />
          </Card>
        </div>

        <Card title="Synthetic Input" flushBody>
          <MetricTable
            rows={[
              { label: "Queue Depth", value: diag.syntheticInput.queueDepth },
              { label: "Total Queued", value: diag.syntheticInput.totalQueued },
              { label: "Peak Depth", value: diag.syntheticInput.peakDepth },
              { label: "Completions", value: diag.syntheticInput.completions },
              { label: "Drops", value: diag.syntheticInput.drops },
            ]}
          />
        </Card>
      </div>
    </div>
  );
}
