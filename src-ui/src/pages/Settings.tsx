import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { NumberInput } from "../components/common/NumberInput";
import { KeyInput } from "../components/common/KeyInput";
import { Dropdown } from "../components/common/Dropdown";
import { Button } from "../components/common/Button";
import { useConfigStore } from "../stores/configStore";
import { useState } from "react";

/// Calibration for points on Dota's HUD that automation clicks.
///
/// Some abilities cannot be self-cast — Dota aims them at the cursor — so the
/// only way to land one on your own hero is to click the hero portrait. That
/// needs a coordinate nobody can derive, hence measuring it once here.
function HudAnchorsCard() {
  const hud = useConfigStore((s) => s.config.hud);
  const [status, setStatus] = useState<{ tone: "ok" | "error"; message: string } | null>(null);

  const run = async (command: "capture_hud_portrait" | "test_hud_portrait", success: string) => {
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
    <Card title="HUD Anchors">
      <p className="text-xs text-muted">
        Slark's shard ability — and anything else Dota refuses to self-cast — is
        aimed by clicking your hero portrait. Hover the centre of the portrait
        in-game and press <code>{hud.capture_portrait_key}</code>, or use the
        button below with Dota visible.
      </p>

      <div className="flex items-center gap-2 text-xs">
        <span className={hud.portrait_calibrated ? "text-green-400" : "text-warning"}>
          {hud.portrait_calibrated ? "✓ Portrait calibrated" : "⚠ Portrait not calibrated"}
        </span>
        <span className="font-mono text-muted">
          ({hud.portrait_x_fraction.toFixed(4)}, {hud.portrait_y_fraction.toFixed(4)})
        </span>
      </div>

      {!hud.portrait_calibrated && (
        <p className="text-xs text-muted">
          Until this is calibrated, automation that would click the portrait is
          skipped rather than guessing — a stray click in Dota is a move order.
        </p>
      )}

      <div className="flex gap-2">
        <Button onClick={() => run("capture_hud_portrait", "Portrait captured.")}>
          Capture Now
        </Button>
        <Button
          variant="secondary"
          disabled={!hud.portrait_calibrated}
          onClick={() => run("test_hud_portrait", "Cursor moved to the anchor.")}
        >
          Test
        </Button>
      </div>

      {status && (
        <p className={`text-xs ${status.tone === "ok" ? "text-green-400" : "text-red-400"}`}>
          {status.message}
        </p>
      )}
      <p className="text-xs text-muted">
        Test parks the cursor on the anchor without clicking, so you can see
        exactly where it lands.
      </p>
    </Card>
  );
}

export default function Settings() {
  const config = useConfigStore((s) => s.config);
  const updateConfig = useConfigStore((s) => s.updateConfig);

  return (
    <div className="space-y-6 p-6">
      <h2 className="text-xl font-semibold">Settings</h2>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="Server">
            <NumberInput
              label="GSI Port"
              value={config.server.port}
              onChange={(v) => updateConfig("server", { port: v })}
            />
            <p className="text-xs text-warning">⚠ Restart required after changing port.</p>
          </Card>

          <Card title="Keybindings">
            <div className="grid grid-cols-3 gap-3">
              <KeyInput label="Slot 1" value={config.keybindings.slot0} onChange={(v) => updateConfig("keybindings", { slot0: v })} />
              <KeyInput label="Slot 2" value={config.keybindings.slot1} onChange={(v) => updateConfig("keybindings", { slot1: v })} />
              <KeyInput label="Slot 3" value={config.keybindings.slot2} onChange={(v) => updateConfig("keybindings", { slot2: v })} />
              <KeyInput label="Slot 4" value={config.keybindings.slot3} onChange={(v) => updateConfig("keybindings", { slot3: v })} />
              <KeyInput label="Slot 5" value={config.keybindings.slot4} onChange={(v) => updateConfig("keybindings", { slot4: v })} />
              <KeyInput label="Slot 6" value={config.keybindings.slot5} onChange={(v) => updateConfig("keybindings", { slot5: v })} />
            </div>
            <KeyInput label="Neutral Slot" value={config.keybindings.neutral0} onChange={(v) => updateConfig("keybindings", { neutral0: v })} />
            <KeyInput label="Combo Trigger" value={config.keybindings.combo_trigger} onChange={(v) => updateConfig("keybindings", { combo_trigger: v })} />
            <KeyInput label="Capture HUD Portrait" value={config.hud.capture_portrait_key} onChange={(v) => updateConfig("hud", { capture_portrait_key: v })} />
          </Card>

          <HudAnchorsCard />
        </div>

        <div className="space-y-4">
          <Card title="Rune Alerts">
            <Toggle label="Enable Rune Alerts" checked={config.rune_alerts.enabled} onChange={(v) => updateConfig("rune_alerts", { enabled: v })} />
            <NumberInput label="Alert Lead Time" value={config.rune_alerts.alert_lead_seconds} onChange={(v) => updateConfig("rune_alerts", { alert_lead_seconds: v })} suffix="s" />
            <NumberInput label="Check Interval" value={config.rune_alerts.interval_seconds} onChange={(v) => updateConfig("rune_alerts", { interval_seconds: v })} suffix="s" />
            <Toggle label="Audio Alert" checked={config.rune_alerts.audio_enabled} onChange={(v) => updateConfig("rune_alerts", { audio_enabled: v })} />
          </Card>

          <Card title="Application">
            <Toggle label="Check for Updates on Startup" checked={config.updates.check_on_startup} onChange={(v) => updateConfig("updates", { check_on_startup: v })} />
            <Toggle label="Include Pre-releases" checked={config.updates.include_prereleases} onChange={(v) => updateConfig("updates", { include_prereleases: v })} />
            <Dropdown
              label="Log Level"
              value={config.logging.level}
              options={[
                { value: "debug", label: "Debug" },
                { value: "info", label: "Info" },
                { value: "warn", label: "Warn" },
                { value: "error", label: "Error" },
              ]}
              onChange={(v) => updateConfig("logging", { level: v as "debug" | "info" | "warn" | "error" })}
            />
            <Toggle
              label="Write Log File"
              checked={config.logging.file_enabled}
              onChange={(v) => updateConfig("logging", { file_enabled: v })}
            />
            <p className="text-xs text-muted">
              Writes to <code>%LOCALAPPDATA%\dota2-scripts\logs\</code>, rotated
              daily. This app has no console, so the file is the only record of
              what it did — leave it on if you might need to report a problem.
            </p>
            <p className="text-xs text-warning">⚠ Restart required after changing this.</p>
          </Card>

          <Card title="Advanced" collapsible defaultOpen={false}>
            <Toggle label="Enable Minimap Capture (Experimental)" checked={config.minimap_capture.enabled} onChange={(v) => updateConfig("minimap_capture", { enabled: v })} />
            {config.minimap_capture.enabled && (
              <div className="grid grid-cols-2 gap-3">
                <NumberInput label="X" value={config.minimap_capture.minimap_x} onChange={(v) => updateConfig("minimap_capture", { minimap_x: v })} />
                <NumberInput label="Y" value={config.minimap_capture.minimap_y} onChange={(v) => updateConfig("minimap_capture", { minimap_y: v })} />
                <NumberInput label="Width" value={config.minimap_capture.minimap_width} onChange={(v) => updateConfig("minimap_capture", { minimap_width: v })} />
                <NumberInput label="Height" value={config.minimap_capture.minimap_height} onChange={(v) => updateConfig("minimap_capture", { minimap_height: v })} />
                <NumberInput label="Capture Interval" value={config.minimap_capture.capture_interval_ms} onChange={(v) => updateConfig("minimap_capture", { capture_interval_ms: v })} suffix="ms" />
                <NumberInput label="Sample Every N" value={config.minimap_capture.sample_every_n} onChange={(v) => updateConfig("minimap_capture", { sample_every_n: v })} />
              </div>
            )}
          </Card>

          <Button variant="danger" className="w-full">
            Reset to Defaults
          </Button>
        </div>
      </div>
    </div>
  );
}

