import { useEffect } from "react";
import { Badge, type BadgeTone } from "../components/common/Badge";
import { Card } from "../components/common/Card";
import { NumberInput } from "../components/common/NumberInput";
import { SettingRow } from "../components/common/SettingRow";
import { Slider } from "../components/common/Slider";
import { useConfigStore } from "../stores/configStore";
import { useMinimapStore } from "../stores/minimapStore";
import type { MapZone, ActivityLevel } from "../types/minimap";
import { ZONE_DISPLAY_NAMES, ZONE_ICONS } from "../types/minimap";

const ACTIVITY_TONES: Record<ActivityLevel, BadgeTone> = {
  Quiet: "neutral",
  Active: "info",
  Fight: "warning",
};

const EVENT_BADGES: Record<string, { tone: BadgeTone; label: string }> = {
  FightDetected: { tone: "warning", label: "FIGHT" },
  FightOngoing: { tone: "warning", label: "FIGHT" },
  EnemyRotation: { tone: "info", label: "ROTATE" },
  EnemyGrouping: { tone: "danger", label: "GROUP" },
};

/**
 * The design draws the page's live capture state as a single monospaced strip
 * above the columns — key, then value, no punctuation between fields.
 */
function StatusBar() {
  const status = useMinimapStore((s) => s.status);

  return (
    <div className="flex flex-wrap items-center gap-6 rounded-lg border border-border bg-surface px-4 py-3 font-mono text-xs">
      <span>
        <span className="text-muted">capture </span>
        <span className="text-content capitalize">{status.health}</span>
      </span>
      <span>
        <span className="text-muted">window </span>
        <span className="text-content">{status.windowBindingStatus}</span>
      </span>
      <span>
        <span className="text-muted">interval </span>
        <span className="text-content">{status.captureIntervalMs}ms</span>
      </span>
      {status.lastCaptureDurationMs != null && (
        <span>
          <span className="text-muted">last </span>
          <span className="text-content">{status.lastCaptureDurationMs}ms</span>
        </span>
      )}
      {status.consecutiveFailures > 0 && (
        <span className="text-danger-text">
          ⚠ {status.consecutiveFailures} failures
        </span>
      )}
    </div>
  );
}

function ZoneRow({ zone }: { zone: MapZone }) {
  const zones = useMinimapStore((s) => s.zones);
  const summary = zones.find((z) => z.zone === zone);

  const activity: ActivityLevel = summary?.currentActivity ?? "Quiet";
  const allies = summary ? Math.round(summary.avgAllyCount) : 0;
  const enemies = summary ? Math.round(summary.avgEnemyCount) : 0;

  return (
    <div className="flex items-center gap-3 py-1">
      <span className="text-sm text-muted">{ZONE_ICONS[zone]}</span>
      <span className="flex-1 text-sm text-content">{ZONE_DISPLAY_NAMES[zone]}</span>
      <span className="font-mono text-xs text-success-text">{allies}</span>
      <span className="font-mono text-xs text-danger-text">{enemies}</span>
      <Badge tone={ACTIVITY_TONES[activity]}>{activity}</Badge>
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
    <div className="max-h-48 space-y-1 overflow-y-auto rounded-md bg-sunken p-4 font-mono text-xs">
      {events.length === 0 && (
        <span className="text-muted">No events detected yet…</span>
      )}
      {events.map((evt) => {
        const badge = EVENT_BADGES[evt.type] ?? EVENT_BADGES.FightDetected;
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
            <span className="shrink-0 text-muted">{evt.timestamp}</span>
            <Badge tone={badge.tone} className="h-4 text-[9px] font-bold">
              {badge.label}
            </Badge>
            <span className="truncate text-content">{message}</span>
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
    <div className="space-y-4 p-6">
      <p className="text-subtle">
        Real-time minimap capture and hero detection via colour analysis.
      </p>

      <StatusBar />

      <div className="grid grid-cols-1 items-start gap-4 lg:grid-cols-2">
        {/* Left Column — Configuration */}
        <div className="space-y-4">
          <Card
            title="Capture Settings"
            subtitle="Region of the screen sampled for hero dots"
          >
            <SettingRow
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

          <Card title="Color Thresholds" subtitle="Hue, saturation and value gates per team" collapsible>
            <div className="space-y-4">
              {/* Red (Dire) detection */}
              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <span className="inline-block h-2 w-2 rounded-full bg-danger" />
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
                  <span className="inline-block h-2 w-2 rounded-full bg-success" />
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
          <Card
            title="Zone Activity"
            subtitle="Radiant / Dire dots seen per zone"
            flushBody
          >
            <div className="flex flex-col">
              {ALL_ZONES.map((zone) => (
                <ZoneRow key={zone} zone={zone} />
              ))}
            </div>
          </Card>

          <Card title="Event Feed" flushBody>
            <EventFeed />
          </Card>

          <Card title="Detection Tuning" collapsible>
            <SettingRow
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
