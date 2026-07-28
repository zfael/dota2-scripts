import { useEffect } from "react";
import { Button } from "../components/common/Button";
import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { NumberInput } from "../components/common/NumberInput";
import { Slider } from "../components/common/Slider";
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

/**
 * Explains an empty map.
 *
 * Without this, "no dots" looks identical whether the game has not started, the
 * fetch is failing, or lane geometry never loaded — which is exactly the
 * ambiguity that made the frozen-snapshot bug hard to spot.
 */
function MapState() {
  const snapshot = useWaveStore((s) => s.snapshot);
  const lanePaths = useWaveStore((s) => s.lanePaths);
  const error = useWaveStore((s) => s.error);
  const connected = useGameStore((s) => s.game.connected);
  const trackingEnabled = useConfigStore((s) => s.config.wave_tracker.enabled);

  let message: string | null = null;
  let tone = "text-muted";

  if (error) {
    message = `Wave data unavailable: ${error}`;
    tone = "text-red-400";
  } else if (!trackingEnabled) {
    message = "Wave tracking is off — enable it under Calibration.";
  } else if (!connected) {
    message = "Waiting for GSI — start a match to see waves.";
  } else if (lanePaths.length === 0) {
    message = "Lane geometry did not load. Restart the app.";
    tone = "text-red-400";
  } else if (snapshot && snapshot.currentWaveAgeSeconds === null) {
    message = "Before the horn — the first wave spawns at 0:00.";
  } else if (!snapshot) {
    message = "Waiting for the first wave snapshot…";
  }

  if (!message) return null;

  return <p className={`text-xs ${tone}`}>{message}</p>;
}

function OverlayControls() {
  const status = useWaveStore((s) => s.overlayStatus);
  const toggleOverlay = useWaveStore((s) => s.toggleOverlay);

  const overlay = useConfigStore((s) => s.config.wave_overlay);
  const updateOverlay = (updates: Partial<typeof overlay>) =>
    useConfigStore.getState().updateConfig("wave_overlay", updates);

  const dotaFound = status != null && status.dotaWindowMode !== "NotFound";

  return (
    <>
      <Toggle
        label="Enable Overlay Hotkey"
        checked={overlay.enabled}
        onChange={(v) => updateOverlay({ enabled: v })}
      />
      <p className="text-xs text-muted">
        Draws wave dots on a transparent, click-through window placed over Dota's
        minimap. Clicks pass through, so minimap click-to-move still works.
      </p>

      <div className="flex items-center gap-3">
        <Button onClick={() => void toggleOverlay()} disabled={!dotaFound}>
          {status?.visible ? "Hide Overlay" : "Show Overlay"}
        </Button>
        <span className="font-mono text-xs text-subtle">
          Hotkey: {status?.toggleKey ?? overlay.toggle_key}
        </span>
      </div>

      <div className="space-y-1 text-xs">
        <div className="text-subtle">
          Dota window:{" "}
          <span className="font-mono text-content">
            {status?.dotaWindowMode ?? "unknown"}
          </span>
        </div>
        {status?.bounds ? (
          <div className="text-subtle">
            Placement:{" "}
            <span className="font-mono text-content">
              {status.bounds.width}×{status.bounds.height} at {status.bounds.x},
              {status.bounds.y}
            </span>
          </div>
        ) : (
          <div className="text-muted">
            Placement unavailable — start Dota 2, and check the minimap region on the
            Minimap page.
          </div>
        )}
      </div>

      <p className="rounded bg-elevated px-3 py-2 text-xs text-subtle">
        Overlays cannot draw over exclusive fullscreen. If the overlay does not
        appear, set Dota to <span className="text-content">Borderless</span> or{" "}
        <span className="text-content">Windowed</span>. This cannot be detected
        reliably from outside the game, so the mode above reports the window style
        only.
      </p>

      <div className="grid grid-cols-2 gap-3">
        <NumberInput
          label="Offset X"
          value={overlay.offset_x}
          onChange={(v) => updateOverlay({ offset_x: v })}
          suffix="px"
        />
        <NumberInput
          label="Offset Y"
          value={overlay.offset_y}
          onChange={(v) => updateOverlay({ offset_y: v })}
          suffix="px"
        />
      </div>
      <Slider
        label="Opacity"
        value={overlay.opacity}
        min={0.1}
        max={1}
        step={0.05}
        onChange={(v) => updateOverlay({ opacity: v })}
      />

      <Toggle
        label="Draw Lane Lines"
        checked={overlay.show_lane_lines}
        onChange={(v) => updateOverlay({ show_lane_lines: v })}
      />
      <p className="text-xs text-muted">
        Off by default: Dota's minimap already draws the lanes and river underneath,
        so the dots alone read more cleanly. Affects the overlay only — the panel on
        this page always shows its lines, having no map behind them.
      </p>

      <Alignment />
    </>
  );
}

/**
 * Live alignment controls for the overlay.
 *
 * The overlay window covers Dota's whole minimap panel, but the map texture is
 * inset inside its bezel — and by how much depends on resolution and Dota's UI
 * scale, so it cannot be a constant. The shipped defaults were measured on one
 * setup; these controls are how anyone else lands on theirs.
 */
function Alignment() {
  const overlay = useConfigStore((s) => s.config.wave_overlay);
  const updateOverlay = (updates: Partial<typeof overlay>) =>
    useConfigStore.getState().updateConfig("wave_overlay", updates);

  // Percent in the UI, fractions in the config: whole-number nudges are far
  // easier to step through than 0.005 increments.
  const asPercent = (value: number) => Math.round(value * 1000) / 10;
  const fromPercent = (value: number) => value / 100;

  return (
    <div className="space-y-3 rounded border border-border p-3">
      <Toggle
        label="Calibration Mode"
        checked={overlay.calibrating}
        onChange={(v) => updateOverlay({ calibrating: v })}
      />
      <p className="text-xs text-muted">
        Shows the overlay's lane lines plus a dashed box around the area it treats
        as the map, with a centre crosshair. Show the overlay, then adjust below
        until the box frames Dota's map and the lines sit on Dota's lanes. Changes
        apply live — no restart.
      </p>

      <div className="grid grid-cols-2 gap-3">
        <NumberInput
          label="Map Offset X"
          value={asPercent(overlay.map_offset_x)}
          min={-25}
          max={25}
          step={0.5}
          onChange={(v) => updateOverlay({ map_offset_x: fromPercent(v) })}
          suffix="%"
        />
        <NumberInput
          label="Map Offset Y"
          value={asPercent(overlay.map_offset_y)}
          min={-25}
          max={25}
          step={0.5}
          onChange={(v) => updateOverlay({ map_offset_y: fromPercent(v) })}
          suffix="%"
        />
        <NumberInput
          label="Map Width"
          value={asPercent(overlay.map_scale_x)}
          min={50}
          max={150}
          step={0.5}
          onChange={(v) => updateOverlay({ map_scale_x: fromPercent(v) })}
          suffix="%"
        />
        <NumberInput
          label="Map Height"
          value={asPercent(overlay.map_scale_y)}
          min={50}
          max={150}
          step={0.5}
          onChange={(v) => updateOverlay({ map_scale_y: fromPercent(v) })}
          suffix="%"
        />
      </div>
      <p className="text-xs text-muted">
        Offsets move map space within the window (Y is positive downward); widths
        and heights scale it about its centre. Distinct from Offset X/Y above,
        which move the whole window instead.
      </p>
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
            <MapState />
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

          <Card title="Minimap Overlay">
            <OverlayControls />
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
