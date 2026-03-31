import { Card } from "../components/common/Card";
import { useGameStore } from "../stores/gameStore";

function StatusDot({ active, label }: { active: boolean; label: string }) {
  return (
    <div className="flex items-center justify-between rounded-md border border-border bg-base p-3">
      <span className="text-sm text-content">{label}</span>
      <div className="flex items-center gap-2">
        <span className={`h-2.5 w-2.5 rounded-full ${active ? "bg-success" : "bg-danger"}`} />
        <span className={`text-xs font-mono ${active ? "text-success" : "text-danger"}`}>
          {active ? "Active" : "Inactive"}
        </span>
      </div>
    </div>
  );
}

function MetricRow({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="flex items-center justify-between py-1">
      <span className="text-xs text-subtle">{label}</span>
      <span className="font-mono text-xs text-content">{value}</span>
    </div>
  );
}

export default function Diagnostics() {
  const diag = useGameStore((s) => s.diagnostics);

  return (
    <div className="space-y-6 p-6">
      <h2 className="text-xl font-semibold">Diagnostics</h2>

      <div className="grid grid-cols-3 gap-4">
        <StatusDot active={diag.gsiConnected} label="GSI Server" />
        <StatusDot active={diag.keyboardHookActive} label="Keyboard Hook" />
        <StatusDot active={diag.gsiConnected} label="Game State" />
      </div>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="GSI Pipeline">
            <MetricRow label="Events Processed" value={diag.queueMetrics.eventsProcessed} />
            <MetricRow label="Events Dropped" value={diag.queueMetrics.eventsDropped} />
            <MetricRow label="Queue Depth" value={`${diag.queueMetrics.currentQueueDepth} / ${diag.queueMetrics.maxQueueDepth}`} />
          </Card>

          <Card title="Keyboard Hook">
            <MetricRow label="Soul Ring State" value={diag.soulRingState} />
            <MetricRow label="Blocked Keys" value={diag.blockedKeys.join(", ") || "None"} />
          </Card>
        </div>

        <div className="space-y-4">
          <Card title="Synthetic Input">
            <MetricRow label="Queue Depth" value={diag.syntheticInput.queueDepth} />
            <MetricRow label="Total Queued" value={diag.syntheticInput.totalQueued} />
            <MetricRow label="Peak Depth" value={diag.syntheticInput.peakDepth} />
            <MetricRow label="Completions" value={diag.syntheticInput.completions} />
            <MetricRow label="Drops" value={diag.syntheticInput.drops} />
          </Card>
        </div>
      </div>
    </div>
  );
}

