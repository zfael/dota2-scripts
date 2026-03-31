import { useEffect } from "react";
import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { NumberInput } from "../components/common/NumberInput";
import { Slider } from "../components/common/Slider";
import { useConfigStore } from "../stores/configStore";
import { useMinimapStore } from "../stores/minimapStore";
import type { MapZone, ActivityLevel } from "../types/minimap";
import { ZONE_DISPLAY_NAMES, ZONE_ICONS } from "../types/minimap";

const ACTIVITY_STYLES: Record<ActivityLevel, { bg: string; text: string }> = {
  Quiet: { bg: "bg-elevated", text: "text-muted" },
  Active: { bg: "bg-blue-900/40", text: "text-blue-400" },
  Fight: { bg: "bg-gold/20", text: "text-gold" },
};

const EVENT_BADGE_STYLES: Record<string, { bg: string; text: string; label: string }> = {
  FightDetected: { bg: "bg-gold/80", text: "text-base", label: "FIGHT" },
  FightOngoing: { bg: "bg-gold/50", text: "text-base", label: "FIGHT" },
  EnemyRotation: { bg: "bg-blue-600", text: "text-white", label: "ROTATE" },
  EnemyGrouping: { bg: "bg-red-600", text: "text-white", label: "GROUP" },
};

function HealthDot({ health }: { health: string }) {
  const colors: Record<string, string> = {
    healthy: "bg-green-500",
    unhealthy: "bg-red-500",
    idle: "bg-muted",
  };
  return (
    <span className={`inline-block h-2.5 w-2.5 rounded-full ${colors[health] ?? colors.idle}`} />
  );
}

function StatusBar() {
  const status = useMinimapStore((s) => s.status);

  return (
    <div className="flex items-center gap-4 rounded-lg border border-border bg-surface px-4 py-2.5 text-sm">
      <div className="flex items-center gap-2">
        <HealthDot health={status.health} />
        <span className="text-subtle">
          Capture: <span className="font-medium text-content capitalize">{status.health}</span>
        </span>
      </div>
      <span className="text-border">|</span>
      <span className="text-subtle">
        Window:{" "}
        <span className="font-mono text-xs text-content">{status.windowBindingStatus}</span>
      </span>
      <span className="text-border">|</span>
      <span className="text-subtle">
        Interval:{" "}
        <span className="font-mono text-xs text-gold">{status.captureIntervalMs}ms</span>
      </span>
      {status.lastCaptureDurationMs != null && (
        <>
          <span className="text-border">|</span>
          <span className="text-subtle">
            Last:{" "}
            <span className="font-mono text-xs text-content">
              {status.lastCaptureDurationMs}ms
            </span>
          </span>
        </>
      )}
      {status.consecutiveFailures > 0 && (
        <>
          <span className="text-border">|</span>
          <span className="font-mono text-xs text-red-400">
            ⚠ {status.consecutiveFailures} failures
          </span>
        </>
      )}
    </div>
  );
}

function ZoneRow({ zone }: { zone: MapZone }) {
  const zones = useMinimapStore((s) => s.zones);
  const summary = zones.find((z) => z.zone === zone);

  const activity: ActivityLevel = summary?.currentActivity ?? "Quiet";
  const style = ACTIVITY_STYLES[activity];
  const allies = summary ? Math.round(summary.avgAllyCount) : 0;
  const enemies = summary ? Math.round(summary.avgEnemyCount) : 0;

  return (
    <div className="flex items-center justify-between">
      <div className="flex items-center gap-2">
        <span className="text-subtle text-sm">{ZONE_ICONS[zone]}</span>
        <span className="text-content text-sm">{ZONE_DISPLAY_NAMES[zone]}</span>
      </div>
      <div className="flex items-center gap-3">
        <span className="font-mono text-xs text-green-400">{allies} 🟢</span>
        <span className="font-mono text-xs text-red-400">{enemies} 🔴</span>
        <span
          className={`rounded px-2 py-0.5 font-mono text-[10px] font-semibold ${style.bg} ${style.text}`}
        >
          {activity}
        </span>
      </div>
    </div>
  );
}

const ALL_ZONES: MapZone[] = [
  "TopLane",
  "MidLane",
  "BotLane",
  "DireJungle",
  "RadiantJungle",
  "Roshan",
  "Other",
];

function EventFeed() {
  const events = useMinimapStore((s) => s.events);

  return (
    <div className="rounded-md bg-base/50 p-3 font-mono text-xs space-y-1 max-h-48 overflow-y-auto">
      {events.length === 0 && (
        <span className="text-muted">No events detected yet…</span>
      )}
      {events.map((evt) => {
        const badge = EVENT_BADGE_STYLES[evt.type] ?? EVENT_BADGE_STYLES.FightDetected;
        const zoneLabel = ZONE_DISPLAY_NAMES[evt.zone] ?? evt.zone;
        let message: string;
        switch (evt.type) {
          case "FightDetected":
            message = `Fight detected in ${zoneLabel}`;
            break;
          case "FightOngoing":
            message = `Fight ongoing in ${zoneLabel}`;
            break;
          case "EnemyRotation":
            message = `Enemy rotation to ${zoneLabel}`;
            break;
          case "EnemyGrouping":
            message = `${evt.count ?? "?"} enemies grouping in ${zoneLabel}`;
            break;
          default:
            message = `Event in ${zoneLabel}`;
        }
        return (
          <div key={evt.id} className="flex items-center gap-2">
            <span className="text-muted shrink-0">{evt.timestamp}</span>
            <span
              className={`rounded px-1.5 py-px text-[9px] font-bold ${badge.bg} ${badge.text} shrink-0`}
            >
              {badge.label}
            </span>
            <span className="text-content truncate">{message}</span>
          </div>
        );
      })}
    </div>
  );
}

export default function MinimapIntelligence() {
  const capture = useConfigStore((s) => s.config.minimap_capture);
  const analysis = useConfigStore((s) => s.config.minimap_analysis);
  const updateCapture = (updates: Partial<typeof capture>) =>
    useConfigStore.getState().updateConfig("minimap_capture", updates);
  const updateAnalysis = (updates: Partial<typeof analysis>) =>
    useConfigStore.getState().updateConfig("minimap_analysis", updates);

  const startPolling = useMinimapStore((s) => s.startPolling);

  useEffect(() => {
    const stop = startPolling();
    return stop;
  }, [startPolling]);

  return (
    <div className="space-y-6 p-6">
      <div>
        <h2 className="text-xl font-semibold">Minimap Intelligence</h2>
        <p className="mt-1 text-sm text-subtle">
          Real-time minimap capture and hero detection via color analysis
        </p>
      </div>

      <StatusBar />

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        {/* Left Column — Configuration */}
        <div className="space-y-4">
          <Card title="Capture Settings">
            <Toggle
              label="Enable Capture"
              checked={capture.enabled}
              onChange={(v) => updateCapture({ enabled: v })}
            />
            <div className="grid grid-cols-2 gap-3">
              <NumberInput
                label="Region X"
                value={capture.minimap_x}
                min={0}
                onChange={(v) => updateCapture({ minimap_x: v })}
              />
              <NumberInput
                label="Region Y"
                value={capture.minimap_y}
                min={0}
                onChange={(v) => updateCapture({ minimap_y: v })}
              />
              <NumberInput
                label="Width"
                value={capture.minimap_width}
                min={1}
                onChange={(v) => updateCapture({ minimap_width: v })}
                suffix="px"
              />
              <NumberInput
                label="Height"
                value={capture.minimap_height}
                min={1}
                onChange={(v) => updateCapture({ minimap_height: v })}
                suffix="px"
              />
            </div>
            <Slider
              label="Capture Interval"
              value={capture.capture_interval_ms}
              min={100}
              max={5000}
              step={100}
              onChange={(v) => updateCapture({ capture_interval_ms: v })}
              suffix="ms"
            />
            <NumberInput
              label="Sample Every N"
              value={capture.sample_every_n}
              min={1}
              max={100}
              onChange={(v) => updateCapture({ sample_every_n: v })}
            />
          </Card>

          <Card title="Color Thresholds" collapsible>
            <div className="space-y-4">
              {/* Red (Dire) detection */}
              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <span className="inline-block h-2 w-2 rounded-full bg-red-500" />
                  <span className="text-xs font-semibold tracking-wider text-subtle uppercase">
                    Dire (Red) Detection
                  </span>
                </div>
                <div className="grid grid-cols-2 gap-3">
                  <NumberInput
                    label="Hue Max"
                    value={analysis.red_hue_max}
                    min={0}
                    max={360}
                    onChange={(v) => updateAnalysis({ red_hue_max: v })}
                  />
                  <NumberInput
                    label="Hue Min Wrap"
                    value={analysis.red_hue_min_wrap}
                    min={0}
                    max={360}
                    onChange={(v) => updateAnalysis({ red_hue_min_wrap: v })}
                  />
                  <NumberInput
                    label="Min Saturation"
                    value={analysis.red_min_saturation}
                    min={0}
                    max={100}
                    onChange={(v) => updateAnalysis({ red_min_saturation: v })}
                  />
                  <NumberInput
                    label="Min Value"
                    value={analysis.red_min_value}
                    min={0}
                    max={100}
                    onChange={(v) => updateAnalysis({ red_min_value: v })}
                  />
                </div>
              </div>

              {/* Green (Radiant) detection */}
              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <span className="inline-block h-2 w-2 rounded-full bg-green-500" />
                  <span className="text-xs font-semibold tracking-wider text-subtle uppercase">
                    Radiant (Green) Detection
                  </span>
                </div>
                <div className="grid grid-cols-2 gap-3">
                  <NumberInput
                    label="Hue Min"
                    value={analysis.green_hue_min}
                    min={0}
                    max={360}
                    onChange={(v) => updateAnalysis({ green_hue_min: v })}
                  />
                  <NumberInput
                    label="Hue Max"
                    value={analysis.green_hue_max}
                    min={0}
                    max={360}
                    onChange={(v) => updateAnalysis({ green_hue_max: v })}
                  />
                  <NumberInput
                    label="Min Saturation"
                    value={analysis.green_min_saturation}
                    min={0}
                    max={100}
                    onChange={(v) => updateAnalysis({ green_min_saturation: v })}
                  />
                  <NumberInput
                    label="Min Value"
                    value={analysis.green_min_value}
                    min={0}
                    max={100}
                    onChange={(v) => updateAnalysis({ green_min_value: v })}
                  />
                </div>
              </div>
            </div>
          </Card>

          <Card title="Baseline Filtering" collapsible>
            <p className="text-xs text-muted">
              Static UI elements (towers, camps) are filtered by accumulating
              frames and removing persistent pixels.
            </p>
            <div className="grid grid-cols-2 gap-3">
              <NumberInput
                label="Baseline Frames"
                value={analysis.baseline_frames}
                min={1}
                max={100}
                onChange={(v) => updateAnalysis({ baseline_frames: v })}
              />
              <NumberInput
                label="Threshold"
                value={analysis.baseline_threshold}
                min={0}
                max={1}
                onChange={(v) => updateAnalysis({ baseline_threshold: v })}
              />
            </div>
          </Card>
        </div>

        {/* Right Column — Live Data */}
        <div className="space-y-4">
          <Card title="Zone Activity">
            <div className="space-y-2.5">
              {ALL_ZONES.map((zone) => (
                <ZoneRow key={zone} zone={zone} />
              ))}
            </div>
          </Card>

          <Card title="Event Feed">
            <EventFeed />
          </Card>

          <Card title="Detection Tuning" collapsible>
            <Toggle
              label="Enable Analysis"
              checked={analysis.enabled}
              onChange={(v) => updateAnalysis({ enabled: v })}
            />
            <div className="grid grid-cols-2 gap-3">
              <NumberInput
                label="Min Cluster Size"
                value={analysis.min_cluster_size}
                min={1}
                max={1000}
                onChange={(v) => updateAnalysis({ min_cluster_size: v })}
              />
              <NumberInput
                label="Max Cluster Size"
                value={analysis.max_cluster_size}
                min={1}
                max={5000}
                onChange={(v) => updateAnalysis({ max_cluster_size: v })}
              />
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}
