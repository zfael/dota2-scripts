import { useEffect } from "react";
import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { NumberInput } from "../components/common/NumberInput";
import { WaveMap } from "../components/waves/WaveMap";
import { useConfigStore } from "../stores/configStore";
import { useGameStore } from "../stores/gameStore";
import { useWaveStore } from "../stores/waveStore";
import type { Lane, WaveConfidence } from "../types/waves";
import { CONFIDENCE_LABELS, LANE_ROLES, formatGameClock } from "../types/waves";

const CONFIDENCE_STYLES: Record<WaveConfidence, string> = {
  High: "bg-green-900/40 text-green-400",
  Degrading: "bg-yellow-900/40 text-yellow-400",
  Low: "bg-elevated text-muted",
};

const ALL_LANES: Lane[] = ["Top", "Mid", "Bottom"];

function StatusBar() {
  const connected = useGameStore((s) => s.game.connected);
  const gameTime = useGameStore((s) => s.game.gameTime);
  const snapshot = useWaveStore((s) => s.snapshot);

  return (
    <div className="flex items-center gap-4 rounded-lg border border-border bg-surface px-4 py-2.5 text-sm">
      <div className="flex items-center gap-2">
        <span
          className={`inline-block h-2.5 w-2.5 rounded-full ${
            connected ? "bg-green-500" : "bg-muted"
          }`}
        />
        <span className="text-subtle">
          GSI: <span className="font-medium text-content">{connected ? "live" : "waiting"}</span>
        </span>
      </div>
      <span className="text-border">|</span>
      <span className="text-subtle">
        Clock: <span className="font-mono text-xs text-content">{formatGameClock(gameTime)}</span>
      </span>
      {snapshot && (
        <>
          <span className="text-border">|</span>
          <span className="text-subtle">
            Next wave:{" "}
            <span className="font-mono text-xs text-gold">
              {snapshot.secondsUntilNextSpawn}s
            </span>
          </span>
          <span className="text-border">|</span>
          <span
            className={`rounded px-2 py-0.5 font-mono text-[10px] font-semibold ${
              CONFIDENCE_STYLES[snapshot.confidence]
            }`}
            title={CONFIDENCE_LABELS[snapshot.confidence]}
          >
            {snapshot.confidence.toUpperCase()}
          </span>
        </>
      )}
    </div>
  );
}

function LaneRow({ lane }: { lane: Lane }) {
  const snapshot = useWaveStore((s) => s.snapshot);
  const clash = snapshot?.clashes.find((c) => c.lane === lane);
  const roles = LANE_ROLES[lane];

  return (
    <div className="flex items-center justify-between">
      <div className="flex items-center gap-2">
        <span className="text-content text-sm">{lane}</span>
        <span className="text-muted text-[10px]">
          R: {roles.radiant} · D: {roles.dire}
        </span>
      </div>
      <span className="font-mono text-xs text-gold">
        {clash
          ? clash.secondsUntilClash > 0
            ? `clash in ${clash.secondsUntilClash}s`
            : "clashing"
          : "—"}
      </span>
    </div>
  );
}

export default function WaveTracker() {
  const lanePaths = useWaveStore((s) => s.lanePaths);
  const snapshot = useWaveStore((s) => s.snapshot);
  const startTracking = useWaveStore((s) => s.startTracking);

  const waves = useConfigStore((s) => s.config.wave_tracker);
  const updateWaves = (updates: Partial<typeof waves>) =>
    useConfigStore.getState().updateConfig("wave_tracker", updates);

  useEffect(() => {
    const stop = startTracking();
    return stop;
  }, [startTracking]);

  return (
    <div className="space-y-6 p-6">
      <div>
        <h2 className="text-xl font-semibold">Wave Tracker</h2>
        <p className="mt-1 text-sm text-subtle">
          Creep wave spawn cadence and predicted clash points, derived from the game clock
        </p>
      </div>

      <StatusBar />

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="Map">
            <div className="aspect-square w-full">
              <WaveMap lanePaths={lanePaths} snapshot={snapshot} />
            </div>
            <p className="text-xs text-muted">
              Positions are predictions, not observations. They hold during laning and
              drift once waves are killed or lanes push — which is what the confidence
              badge tracks.
            </p>
          </Card>
        </div>

        <div className="space-y-4">
          <Card title="Lane Clashes">
            <div className="space-y-2.5">
              {ALL_LANES.map((lane) => (
                <LaneRow key={lane} lane={lane} />
              ))}
            </div>
          </Card>

          <Card title="Calibration" collapsible>
            <Toggle
              label="Enable Wave Tracking"
              checked={waves.enabled}
              onChange={(v) => updateWaves({ enabled: v })}
            />
            <p className="text-xs text-muted">
              Meet times are empirical approximations. Retune them here if waves clash
              earlier or later than shown.
            </p>
            <div className="grid grid-cols-2 gap-3">
              <NumberInput
                label="Mid Meet"
                value={waves.mid_meet_seconds}
                min={1}
                max={30}
                onChange={(v) => updateWaves({ mid_meet_seconds: v })}
                suffix="s"
              />
              <NumberInput
                label="Side Meet"
                value={waves.side_meet_seconds}
                min={1}
                max={30}
                onChange={(v) => updateWaves({ side_meet_seconds: v })}
                suffix="s"
              />
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}
