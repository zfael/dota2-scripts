import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { NumberInput } from "../components/common/NumberInput";
import { KeyInput } from "../components/common/KeyInput";
import { Dropdown } from "../components/common/Dropdown";
import { Button } from "../components/common/Button";
import { useConfigStore } from "../stores/configStore";

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
          </Card>

          <Card title="Common">
            <NumberInput
              label="Survivability HP Threshold"
              value={config.common.survivability_hp_threshold}
              onChange={(v) => updateConfig("common", { survivability_hp_threshold: v })}
              suffix="%"
            />
          </Card>
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

