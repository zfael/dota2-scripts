import { useEffect } from "react";
import { Alert } from "../components/common/Alert";
import { Badge, type BadgeTone } from "../components/common/Badge";
import { Button } from "../components/common/Button";
import { Card } from "../components/common/Card";
import { NumberInput } from "../components/common/NumberInput";
import { SettingRow } from "../components/common/SettingRow";
import { Slider } from "../components/common/Slider";
import { WaveMap } from "../components/waves/WaveMap";
import { useConfigStore } from "../stores/configStore";
import { useGameStore } from "../stores/gameStore";
import { useWaveStore } from "../stores/waveStore";
import type { Lane, WaveConfidence } from "../types/waves";
import { CONFIDENCE_LABELS, LANE_ROLES, formatGameClock } from "../types/waves";

const CONFIDENCE_TONES: Record<WaveConfidence, BadgeTone> = {
  High: "success",
  Degrading: "warning",
  Low: "neutral",
};

const ALL_LANES: Lane[] = ["Top", "Mid", "Bottom"];

function StatusBar() {
  const connected = useGameStore((s) => s.game.connected);
  const gameTime = useGameStore((s) => s.game.gameTime);
  const snapshot = useWaveStore((s) => s.snapshot);

  return (
    <div className="flex flex-wrap items-center gap-6 rounded-lg border border-border bg-surface px-4 py-3 font-mono text-xs">
      <span>
        <span className="text-muted">gsi </span>
        <span className="text-content">{connected ? "live" : "waiting"}</span>
      </span>
      <span>
        <span className="text-muted">clock </span>
        <span className="text-content">{formatGameClock(gameTime)}</span>
      </span>
      {snapshot && (
        <>
          <span>
            <span className="text-muted">next wave </span>
            <span className="text-content">{snapshot.secondsUntilNextSpawn}s</span>
          </span>
          <Badge
            tone={CONFIDENCE_TONES[snapshot.confidence]}
            title={CONFIDENCE_LABELS[snapshot.confidence]}
          >
            {snapshot.confidence.toUpperCase()}
          </Badge>
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
    <div className="flex items-center gap-3">
      <span className="w-16 shrink-0 text-sm font-medium text-content">{lane}</span>
      <span className="flex-1 font-mono text-2xs text-muted">
        R: {roles.radiant} · D: {roles.dire}
      </span>
      <span className="font-mono text-xs text-subtle">
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
    tone = "text-danger-text";
  } else if (!trackingEnabled) {
    message = "Wave tracking is off — enable it under Calibration.";
  } else if (!connected) {
    message = "Waiting for GSI — start a match to see waves.";
  } else if (lanePaths.length === 0) {
    message = "Lane geometry did not load. Restart the app.";
    tone = "text-danger-text";
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
      <SettingRow
        label="Enable Overlay Hotkey"
        description="Draws wave dots on a transparent, click-through window placed over Dota's minimap. Clicks pass through, so minimap click-to-move still works."
        checked={overlay.enabled}
        onChange={(v) => updateOverlay({ enabled: v })}
      />

      <div className="flex items-center gap-3">
        <Button
          variant="secondary"
          size="sm"
          onClick={() => void toggleOverlay()}
          disabled={!dotaFound}
        >
          {status?.visible ? "Hide Overlay" : "Show Overlay"}
        </Button>
        <span className="font-mono text-xs text-muted">
          Hotkey {status?.toggleKey ?? overlay.toggle_key}
        </span>
      </div>

      <div className="flex flex-col gap-1.5 font-mono text-xs text-subtle">
        <div>
          <span className="text-muted">dota window </span>
          {status?.dotaWindowMode ?? "unknown"}
        </div>
        {status?.bounds ? (
          <div>
            <span className="text-muted">placement </span>
            {status.bounds.width}×{status.bounds.height} at {status.bounds.x},
            {status.bounds.y}
          </div>
        ) : (
          <div className="text-muted">
            placement unavailable — start Dota 2, and check the minimap region on
            the Minimap page.
          </div>
        )}
      </div>

      <Alert tone="warning">
        Overlays cannot draw over exclusive fullscreen. If the overlay does not
        appear, set Dota to <span className="text-content">Borderless</span> or{" "}
        <span className="text-content">Windowed</span>. This cannot be detected
        reliably from outside the game, so the mode above reports the window style
        only.
      </Alert>

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

      <SettingRow
        label="Draw Lane Lines"
        description="Off by default: Dota's minimap already draws the lanes and river underneath, so the dots alone read more cleanly. Affects the overlay only — the panel on this page always shows its lines, having no map behind them."
        checked={overlay.show_lane_lines}
        onChange={(v) => updateOverlay({ show_lane_lines: v })}
      />

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
    <div className="space-y-4 rounded-md border border-border bg-sunken p-3">
      <SettingRow
        label="Calibration Mode"
        description="Shows the overlay's lane lines plus a dashed box around the area it treats as the map, with a centre crosshair. Show the overlay, then adjust below until the box frames Dota's map and the lines sit on Dota's lanes. Changes apply live — no restart."
        checked={overlay.calibrating}
        onChange={(v) => updateOverlay({ calibrating: v })}
      />

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
    <div className="space-y-4 p-6">
      <p className="text-subtle">
        Creep wave spawn cadence and predicted clash points, derived from the game
        clock.
      </p>

      <StatusBar />

      <div className="grid grid-cols-1 items-start gap-4 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="Map" subtitle="Positions are predictions, not observations">
            <div className="aspect-square w-full">
              <WaveMap lanePaths={lanePaths} snapshot={snapshot} />
            </div>
            <MapState />
            <p className="text-xs leading-relaxed text-muted">
              They hold during laning and drift once waves are killed or lanes push —
              which is what the confidence badge tracks.
            </p>
          </Card>
        </div>

        <div className="space-y-4">
          <Card title="Lane Clashes">
            {ALL_LANES.map((lane) => (
              <LaneRow key={lane} lane={lane} />
            ))}
          </Card>

          <Card
            title="Minimap Overlay"
            subtitle="Click-through window drawn over Dota's minimap"
          >
            <OverlayControls />
          </Card>

          <Card title="Calibration" collapsible>
            <SettingRow
              label="Enable Wave Tracking"
              description="Meet times are empirical approximations. Retune them here if waves clash earlier or later than shown."
              checked={waves.enabled}
              onChange={(v) => updateWaves({ enabled: v })}
            />
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
