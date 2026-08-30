import { useState } from "react";
import { Alert } from "../components/common/Alert";
import { Badge } from "../components/common/Badge";
import { Button } from "../components/common/Button";
import { Card } from "../components/common/Card";
import { Dropdown } from "../components/common/Dropdown";
import { KeyInput } from "../components/common/KeyInput";
import { NumberInput } from "../components/common/NumberInput";
import { SettingRow } from "../components/common/SettingRow";
import { useConfigStore } from "../stores/configStore";
import type { KeybindingsConfig } from "../types/config";

/// Calibration for points on Dota's HUD that automation clicks.
///
/// Some abilities cannot be self-cast — Dota aims them at the cursor — so the
/// only way to land one on your own hero is to click the hero portrait. That
/// needs a coordinate nobody can derive, hence measuring it once here.
function HudAnchorsCard() {
  const hud = useConfigStore((s) => s.config.hud);
  const [status, setStatus] = useState<{ tone: "ok" | "error"; message: string } | null>(
    null,
  );

  const run = async (
    command: "capture_hud_portrait" | "test_hud_portrait",
    success: string,
  ) => {
    setStatus(null);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke(command);
      setStatus({ tone: "ok", message: success });
    } catch (e) {
      setStatus({ tone: "error", message: e instanceof Error ? e.message : String(e) });
    }
  };

  return (
    <Card
      title="HUD Anchors"
      subtitle={`Hover the centre of the portrait in-game and press ${hud.capture_portrait_key}`}
      action={
        <Badge tone={hud.portrait_calibrated ? "success" : "warning"} dot>
          {hud.portrait_calibrated ? "Calibrated" : "Not calibrated"}
        </Badge>
      }
    >
      <div className="flex items-start justify-between gap-4">
        <p className="text-xs leading-relaxed text-muted">
          Slark's shard ability — and anything else Dota refuses to self-cast — is
          aimed by clicking your hero portrait.
        </p>
        <span className="shrink-0 font-mono text-2xs text-muted">
          ({hud.portrait_x_fraction.toFixed(4)}, {hud.portrait_y_fraction.toFixed(4)})
        </span>
      </div>

      {!hud.portrait_calibrated && (
        <p className="text-xs leading-relaxed text-muted">
          Until this is calibrated, automation that would click the portrait is
          skipped rather than guessing — a stray click in Dota is a move order.
        </p>
      )}

      <div className="flex gap-2">
        <Button size="sm" onClick={() => run("capture_hud_portrait", "Portrait captured.")}>
          Capture Now
        </Button>
        <Button
          variant="secondary"
          size="sm"
          disabled={!hud.portrait_calibrated}
          onClick={() => run("test_hud_portrait", "Cursor moved to the anchor.")}
        >
          Test
        </Button>
      </div>

      {status && (
        <p
          className={`text-xs ${
            status.tone === "ok" ? "text-success-text" : "text-danger-text"
          }`}
        >
          {status.message}
        </p>
      )}
      <p className="text-xs leading-relaxed text-muted">
        Test parks the cursor on the anchor without clicking, so you can see
        exactly where it lands.
      </p>
    </Card>
  );
}

export default function Settings() {
  const config = useConfigStore((s) => s.config);
  const updateConfig = useConfigStore((s) => s.updateConfig);

  const slots = [
    { label: "Slot 1", key: "slot0" },
    { label: "Slot 2", key: "slot1" },
    { label: "Slot 3", key: "slot2" },
    { label: "Slot 4", key: "slot3" },
    { label: "Slot 5", key: "slot4" },
    { label: "Slot 6", key: "slot5" },
  ] as const;

  return (
    <div className="grid grid-cols-1 items-start gap-4 p-6 lg:grid-cols-2">
      <div className="space-y-4">
        <Card title="Server">
          <NumberInput
            label="GSI Port"
            value={config.server.port}
            onChange={(v) => updateConfig("server", { port: v })}
          />
          <Alert tone="warning">Restart required after changing port.</Alert>
        </Card>

        <Card title="Keybindings" subtitle="Item slots the runtime is allowed to press">
          <div className="grid grid-cols-3 gap-3">
            {slots.map((slot) => (
              <KeyInput
                key={slot.key}
                label={slot.label}
                value={config.keybindings[slot.key]}
                onChange={(v) =>
                  updateConfig("keybindings", {
                    [slot.key]: v,
                  } as Partial<KeybindingsConfig>)
                }
              />
            ))}
          </div>
          <div className="grid grid-cols-3 gap-3">
            <KeyInput
              label="Neutral Slot"
              value={config.keybindings.neutral0}
              onChange={(v) => updateConfig("keybindings", { neutral0: v })}
            />
            <KeyInput
              label="Combo Trigger"
              value={config.keybindings.combo_trigger}
              onChange={(v) => updateConfig("keybindings", { combo_trigger: v })}
            />
            <KeyInput
              label="Capture HUD Portrait"
              value={config.hud.capture_portrait_key}
              onChange={(v) => updateConfig("hud", { capture_portrait_key: v })}
            />
          </div>
        </Card>

        <HudAnchorsCard />
      </div>

      <div className="space-y-4">
        <Card title="Rune Alerts">
          <SettingRow
            label="Enable Rune Alerts"
            checked={config.rune_alerts.enabled}
            onChange={(v) => updateConfig("rune_alerts", { enabled: v })}
          />
          <NumberInput
            label="Alert Lead Time"
            value={config.rune_alerts.alert_lead_seconds}
            onChange={(v) => updateConfig("rune_alerts", { alert_lead_seconds: v })}
            suffix="s"
          />
          <NumberInput
            label="Check Interval"
            value={config.rune_alerts.interval_seconds}
            onChange={(v) => updateConfig("rune_alerts", { interval_seconds: v })}
            suffix="s"
          />
          <SettingRow
            label="Audio Alert"
            checked={config.rune_alerts.audio_enabled}
            onChange={(v) => updateConfig("rune_alerts", { audio_enabled: v })}
          />
        </Card>

        <Card title="Application">
          <SettingRow
            label="Check for Updates on Startup"
            checked={config.updates.check_on_startup}
            onChange={(v) => updateConfig("updates", { check_on_startup: v })}
          />
          <SettingRow
            label="Include Pre-releases"
            checked={config.updates.include_prereleases}
            onChange={(v) => updateConfig("updates", { include_prereleases: v })}
          />
          <Dropdown
            label="Log Level"
            value={config.logging.level}
            options={[
              { value: "debug", label: "Debug" },
              { value: "info", label: "Info" },
              { value: "warn", label: "Warn" },
              { value: "error", label: "Error" },
            ]}
            onChange={(v) =>
              updateConfig("logging", {
                level: v as "debug" | "info" | "warn" | "error",
              })
            }
          />
          <SettingRow
            label="Write Log File"
            description="Writes to %LOCALAPPDATA%\dota2-scripts\logs\, rotated daily. This app has no console, so the file is the only record of what it did — leave it on if you might need to report a problem."
            checked={config.logging.file_enabled}
            onChange={(v) => updateConfig("logging", { file_enabled: v })}
          />
          <Alert tone="warning">Restart required after changing this.</Alert>
        </Card>

        <Card
          title="Advanced"
          subtitle="Restores every value in config.toml to its shipped default"
          footer={
            <Button variant="danger" size="sm">
              Reset to Defaults
            </Button>
          }
          flushBody
        >
          <p className="text-xs leading-relaxed text-muted">
            Minimap capture and colour thresholds moved to{" "}
            <span className="text-content">Minimap Intelligence</span>.
          </p>
        </Card>
      </div>
    </div>
  );
}
