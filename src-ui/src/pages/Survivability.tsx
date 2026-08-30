import { Link } from "react-router-dom";
import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { Slider } from "../components/common/Slider";
import { NumberInput } from "../components/common/NumberInput";
import { KeyInput } from "../components/common/KeyInput";
import { TagList } from "../components/common/TagList";
import { SettingRow } from "../components/common/SettingRow";
import { Divider } from "../components/common/Field";
import { useConfigStore } from "../stores/configStore";
import type { DangerDetectionConfig } from "../types/config";

type DefensiveKey =
  | "auto_bkb"
  | "auto_satanic"
  | "auto_blade_mail"
  | "auto_mjollnir"
  | "auto_glimmer_cape"
  | "auto_ghost_scepter"
  | "auto_shivas_guard";

/** Priority order the runtime uses. The number is shown so the page states it. */
const DEFENSIVE_ITEMS: { key: DefensiveKey; name: string }[] = [
  { key: "auto_bkb", name: "Black King Bar" },
  { key: "auto_satanic", name: "Satanic" },
  { key: "auto_blade_mail", name: "Blade Mail" },
  { key: "auto_mjollnir", name: "Mjollnir" },
  { key: "auto_glimmer_cape", name: "Glimmer Cape" },
  { key: "auto_ghost_scepter", name: "Ghost Scepter" },
  { key: "auto_shivas_guard", name: "Shiva's Guard" },
];

export default function Survivability() {
  const common = useConfigStore((s) => s.config.common);
  const danger = useConfigStore((s) => s.config.danger_detection);
  const neutral = useConfigStore((s) => s.config.neutral_items);
  const invisibility = useConfigStore((s) => s.config.invisibility);
  const updateCommon = (updates: Partial<typeof common>) =>
    useConfigStore.getState().updateConfig("common", updates);
  const updateDanger = (updates: Partial<typeof danger>) =>
    useConfigStore.getState().updateConfig("danger_detection", updates);
  const updateNeutral = (updates: Partial<typeof neutral>) =>
    useConfigStore.getState().updateConfig("neutral_items", updates);
  const updateInvisibility = (updates: Partial<typeof invisibility>) =>
    useConfigStore.getState().updateConfig("invisibility", updates);

  const lanePhaseEnabled = common.lane_phase_duration_seconds > 0;

  return (
    <div className="space-y-4 p-6">
      <p className="text-subtle">
        What the runtime does to keep you alive. When it decides you are in danger
        is tuned on{" "}
        <Link to="/danger" className="text-accent-text hover:underline">
          Danger Detection
        </Link>
        ; item-specific automation lives on{" "}
        <Link to="/armlet" className="text-accent-text hover:underline">
          Armlet
        </Link>
        ,{" "}
        <Link to="/boots" className="text-accent-text hover:underline">
          Boots
        </Link>{" "}
        and{" "}
        <Link to="/soul-ring" className="text-accent-text hover:underline">
          Soul Ring
        </Link>
        .
      </p>

      <div className="grid grid-cols-1 items-start gap-4 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="Healing Items">
            <Slider
              label="Healing HP Threshold"
              value={common.survivability_hp_threshold}
              min={5}
              max={90}
              onChange={(v) => updateCommon({ survivability_hp_threshold: v })}
              suffix="%"
              hint="Baseline: heals once per event below this HP."
            />
            <Slider
              label="Healing HP Threshold in Danger"
              value={danger.healing_threshold_in_danger}
              min={30}
              max={80}
              onChange={(v) => updateDanger({ healing_threshold_in_danger: v })}
              suffix="%"
            />
            <Slider
              label="Max Healing Items/Event"
              value={danger.max_healing_items_per_danger}
              min={1}
              max={5}
              onChange={(v) => updateDanger({ max_healing_items_per_danger: v })}
            />
            <div className="flex flex-col gap-1.5 rounded-md bg-sunken p-3 font-mono text-2xs leading-relaxed text-subtle">
              <div>
                <span className="text-muted">normal&nbsp;&nbsp;</span>Cheese → Magic
                Stick → Faerie Fire → Magic Wand → Enchanted Mango → Greater Faerie
                Fire
              </div>
              <div>
                <span className="text-muted">danger&nbsp;&nbsp;</span>Cheese → Greater
                Faerie Fire → Enchanted Mango → Magic Wand → Magic Stick → Faerie
                Fire
              </div>
            </div>
          </Card>

          <Card
            title="Lane Phase"
            subtitle="Stops the runtime burning regen on lane harass"
          >
            <SettingRow
              label="Use a Lower Threshold Early"
              description="Overrides both the normal and danger thresholds while it is active."
              checked={lanePhaseEnabled}
              onChange={(v) =>
                updateCommon({ lane_phase_duration_seconds: v ? 480 : 0 })
              }
            />
            {lanePhaseEnabled && (
              <>
                <Slider
                  label="Lane Phase Healing Threshold"
                  value={common.lane_phase_healing_threshold}
                  min={5}
                  max={60}
                  onChange={(v) =>
                    updateCommon({ lane_phase_healing_threshold: v })
                  }
                  suffix="%"
                />
                <NumberInput
                  label="Lane Phase Duration"
                  value={common.lane_phase_duration_seconds}
                  onChange={(v) =>
                    updateCommon({ lane_phase_duration_seconds: v })
                  }
                  suffix="s"
                  hint="Measured from the horn. Pre-game clock values do not count."
                />
              </>
            )}
          </Card>

          <Card title="Invisibility">
            <SettingRow
              label="Hold Automation While Invisible"
              description="Shadow Blade and Silver Edge invisibility drops the moment anything is cast. While running, this holds Slark's Dark Pact, Phase Boots, healing, defensive, neutral and mana items, and the silence dispels."
              checked={invisibility.suppress_automation}
              onChange={(v) => updateInvisibility({ suppress_automation: v })}
            />
            <p className="text-xs leading-relaxed text-muted">
              Never held: Slark's Shadow Dance and Depth Shroud, which grant
              invisibility rather than ending it, and Soul Ring and Armlet, which
              fire off your own keypress or to stop you dying.
            </p>
          </Card>

          <Card
            title="Dispels"
            subtitle="Fires on silence alone — independent of danger state — at most once per silence, Manta first"
          >
            <Toggle
              label="Auto-Manta on Silence"
              checked={danger.auto_manta_on_silence}
              onChange={(v) => updateDanger({ auto_manta_on_silence: v })}
            />
            <Toggle
              label="Auto-Lotus on Silence"
              checked={danger.auto_lotus_on_silence}
              onChange={(v) => updateDanger({ auto_lotus_on_silence: v })}
            />
          </Card>
        </div>

        <div className="space-y-4">
          <Card
            title="Defensive Items"
            subtitle="Used in priority order, only while danger detection is active"
            flushBody
          >
            <div className="flex flex-col">
              {DEFENSIVE_ITEMS.map((item, i) => (
                <div
                  key={item.key}
                  className="flex items-center justify-between gap-3 py-2"
                >
                  <span className="flex items-center gap-2">
                    <span className="font-mono text-2xs text-muted">{i + 1}</span>
                    <span className="text-sm text-content">{item.name}</span>
                  </span>
                  <Toggle
                    label=""
                    ariaLabel={item.name}
                    checked={danger[item.key]}
                    onChange={(v) =>
                      updateDanger({ [item.key]: v } as Partial<DangerDetectionConfig>)
                    }
                  />
                </div>
              ))}
            </div>
            {danger.auto_satanic && (
              <>
                <Divider className="my-3" />
                <Slider
                  label="Satanic HP Threshold"
                  value={danger.satanic_hp_threshold}
                  min={10}
                  max={70}
                  onChange={(v) => updateDanger({ satanic_hp_threshold: v })}
                  suffix="%"
                />
              </>
            )}
          </Card>

          <Card title="Neutral Items">
            <SettingRow
              label="Enable"
              checked={neutral.enabled}
              onChange={(v) => updateNeutral({ enabled: v })}
            />
            <SettingRow
              label="Use in Danger Only"
              checked={neutral.use_in_danger}
              onChange={(v) => updateNeutral({ use_in_danger: v })}
            />
            <Slider
              label="Neutral HP Threshold"
              value={neutral.hp_threshold}
              min={10}
              max={90}
              onChange={(v) => updateNeutral({ hp_threshold: v })}
              suffix="%"
            />
            <KeyInput
              label="Self-Cast Key"
              value={neutral.self_cast_key}
              onChange={(v) => updateNeutral({ self_cast_key: v })}
            />
            <TagList
              label="Allowed Items"
              items={neutral.allowed_items}
              onChange={(v) => updateNeutral({ allowed_items: v })}
            />
            <p className="text-xs leading-relaxed text-muted">
              Listed neutrals the runtime has no cast mode for are ignored at
              runtime.
            </p>
          </Card>
        </div>
      </div>
    </div>
  );
}
