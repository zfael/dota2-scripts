import { useEffect } from "react";
import { Button } from "../components/common/Button";
import { Card } from "../components/common/Card";
import { Dropdown } from "../components/common/Dropdown";
import { Toggle } from "../components/common/Toggle";
import { NumberInput } from "../components/common/NumberInput";
import { Slider } from "../components/common/Slider";
import { SettingRow } from "../components/common/SettingRow";
import { useAlertStore } from "../stores/alertStore";
import { useConfigStore } from "../stores/configStore";
import type { AlertEventKey } from "../types/alerts";
import { ALERT_EVENTS, formatCountdown } from "../types/alerts";
import type { AlertEventConfig } from "../types/config";

function EventRow({
  eventKey,
  label,
  schedule,
  cue,
}: {
  eventKey: AlertEventKey;
  label: string;
  schedule: string;
  cue: string;
}) {
  const settings = useConfigStore((s) => s.config.alerts[eventKey]);
  const countdown = useAlertStore((s) =>
    s.countdowns.find((c) => c.event === eventKey),
  );
  const testPlay = useAlertStore((s) => s.testPlay);

  const update = (updates: Partial<AlertEventConfig>) =>
    useConfigStore
      .getState()
      .updateConfig("alerts", { [eventKey]: { ...settings, ...updates } });

  return (
    <div className="rounded-md border border-border bg-elevated p-3">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className="font-medium text-content">{label}</span>
            <span className="font-mono text-xs text-accent-text">
              {formatCountdown(countdown?.secondsUntil ?? null)}
            </span>
          </div>
          <div className="font-mono text-2xs text-muted">{schedule}</div>
          <div className="text-xs text-subtle">{cue}</div>
        </div>
        <div className="flex shrink-0 items-center gap-3">
          <Button variant="soft" size="sm" onClick={() => void testPlay(eventKey)}>
            Test
          </Button>
          <Toggle
            label=""
            ariaLabel={label}
            checked={settings.enabled}
            onChange={(v) => update({ enabled: v })}
          />
        </div>
      </div>

      <div className="mt-3 grid grid-cols-2 gap-3">
        <NumberInput
          label="Lead Time"
          value={settings.lead_seconds}
          min={0}
          max={120}
          onChange={(v) => update({ lead_seconds: v })}
          suffix="s"
        />
        <Slider
          label="Volume"
          value={settings.volume}
          min={0}
          max={1}
          step={0.05}
          onChange={(v) => update({ volume: v })}
        />
      </div>
    </div>
  );
}

export default function Alerts() {
  const alerts = useConfigStore((s) => s.config.alerts);
  const voicePacks = useAlertStore((s) => s.voicePacks);
  const startPolling = useAlertStore((s) => s.startPolling);

  const updateAlerts = (updates: Partial<typeof alerts>) =>
    useConfigStore.getState().updateConfig("alerts", updates);

  useEffect(() => {
    const stop = startPolling();
    return stop;
  }, [startPolling]);

  return (
    <div className="max-w-[900px] space-y-4 p-6">
      <p className="text-subtle">
        Audio cues for runes, Tormentor, neutral drops, and stack timings.
      </p>

      <Card title="Master">
        <SettingRow
          label="Enable Alerts"
          checked={alerts.enabled}
          onChange={(v) => updateAlerts({ enabled: v })}
        />
        <Slider
          label="Master Volume"
          value={alerts.master_volume}
          min={0}
          max={1}
          step={0.05}
          onChange={(v) => updateAlerts({ master_volume: v })}
        />
        <p className="text-xs leading-relaxed text-muted">
          Cues are generated in the app, so they keep working with the window
          minimised. Each event has a distinct rhythm — the pulse count matches how
          often it happens, so two blips is the 2-minute power rune and three notes
          is the 7-minute wisdom rune.
        </p>

        <Dropdown
          label="Voice Pack"
          value={alerts.voice_pack}
          options={[
            { value: "", label: "Generated cues (no voice)" },
            ...voicePacks.map((pack) => ({ value: pack, label: pack })),
          ]}
          onChange={(v) => updateAlerts({ voice_pack: v })}
          hint="A voice pack replaces the cues with spoken callouts. Packs are not shipped — generate one with scripts/generate-voice-pack.ps1, or drop your own files in assets/voice/<name>/. Any event the pack is missing falls back to its generated cue."
        />
      </Card>

      <Card title="Events" flushBody>
        <div className="flex flex-col gap-3">
          {ALERT_EVENTS.map((event) => (
            <EventRow
              key={event.key}
              eventKey={event.key}
              label={event.label}
              schedule={event.schedule}
              cue={event.cue}
            />
          ))}
        </div>
      </Card>
    </div>
  );
}
