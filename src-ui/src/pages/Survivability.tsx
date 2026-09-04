import { Link } from "react-router-dom";
import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { Slider } from "../components/common/Slider";
import { NumberInput } from "../components/common/NumberInput";
import { KeyInput } from "../components/common/KeyInput";
import { TagList } from "../components/common/TagList";
import { useConfigStore } from "../stores/configStore";

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
    <div className="space-y-6 p-6">
      <div className="space-y-1">
        <h2 className="text-xl font-semibold">Survivability</h2>
        <p className="text-xs text-subtle">
          What the runtime does to keep you alive. When it decides you are in
          danger is tuned on{" "}
          <Link to="/danger" className="text-gold hover:underline">
            Danger Detection
          </Link>
          ; item-specific automation lives on{" "}
          <Link to="/armlet" className="text-gold hover:underline">
            Armlet
          </Link>
          ,{" "}
          <Link to="/boots" className="text-gold hover:underline">
            Boots
          </Link>{" "}
          and{" "}
          <Link to="/soul-ring" className="text-gold hover:underline">
            Soul Ring
          </Link>
          .
        </p>
      </div>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="Healing Items">
            <Slider
              label="Healing HP Threshold"
              value={common.survivability_hp_threshold}
              min={5}
              max={90}
              onChange={(v) => updateCommon({ survivability_hp_threshold: v })}
              suffix="%"
            />
            <p className="text-xs text-subtle">
              Baseline: heals once per event below this HP.
            </p>
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
            <div className="mt-2 space-y-1 text-xs text-muted">
              <p className="font-medium text-subtle">
                Normal: Cheese → Magic Stick → Faerie Fire → Magic Wand →
                Enchanted Mango → Greater Faerie Fire
              </p>
              <p className="font-medium text-subtle">
                In danger: Cheese → Greater Faerie Fire → Enchanted Mango →
                Magic Wand → Magic Stick → Faerie Fire
              </p>
            </div>
          </Card>

          <Card title="Lane Phase">
            <Toggle
              label="Use a Lower Threshold Early"
              checked={lanePhaseEnabled}
              onChange={(v) =>
                updateCommon({ lane_phase_duration_seconds: v ? 480 : 0 })
              }
            />
            <p className="text-xs text-subtle">
              Stops the runtime from burning regen on lane harass. Overrides both
              the normal and danger thresholds while it is active.
            </p>
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
                />
                <p className="text-xs text-subtle">
                  Measured from the horn. Pre-game clock values do not count.
                </p>
              </>
            )}
          </Card>

          <Card title="Invisibility">
            <Toggle
              label="Hold Automation While Invisible"
              checked={invisibility.suppress_automation}
              onChange={(v) => updateInvisibility({ suppress_automation: v })}
            />
            <p className="text-xs text-subtle">
              Shadow Blade and Silver Edge invisibility drops the moment anything
              is cast or activated. While it is running, this holds Slark's Dark
              Pact, Phase Boots, healing, defensive, neutral and mana items, and
              the silence dispels.
            </p>
            <p className="text-xs text-muted">
              Never held: Slark's Shadow Dance and Depth Shroud, which grant
              invisibility rather than ending it, and Soul Ring and Armlet, which
              fire off your own keypress or to stop you dying.
            </p>
          </Card>

          <Card title="Dispels">
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
            <p className="text-xs text-subtle">
              Fires on silence alone — independent of danger state — at most once
              per silence, Manta first.
            </p>
          </Card>
        </div>

        <div className="space-y-4">
          <Card title="Defensive Items">
            <p className="text-xs text-subtle">
              Used in priority order, only while danger detection is active.
            </p>
            <Toggle
              label="Black King Bar"
              checked={danger.auto_bkb}
              onChange={(v) => updateDanger({ auto_bkb: v })}
            />
            <Toggle
              label="Satanic"
              checked={danger.auto_satanic}
              onChange={(v) => updateDanger({ auto_satanic: v })}
            />
            {danger.auto_satanic && (
              <Slider
                label="Satanic HP Threshold"
                value={danger.satanic_hp_threshold}
                min={10}
                max={70}
                onChange={(v) => updateDanger({ satanic_hp_threshold: v })}
                suffix="%"
              />
            )}
            <Toggle
              label="Blade Mail"
              checked={danger.auto_blade_mail}
              onChange={(v) => updateDanger({ auto_blade_mail: v })}
            />
            <Toggle
              label="Lotus Orb"
              checked={danger.auto_lotus_orb}
              onChange={(v) => updateDanger({ auto_lotus_orb: v })}
            />
            <Toggle
              label="Mjollnir"
              checked={danger.auto_mjollnir}
              onChange={(v) => updateDanger({ auto_mjollnir: v })}
            />
            <Toggle
              label="Glimmer Cape"
              checked={danger.auto_glimmer_cape}
              onChange={(v) => updateDanger({ auto_glimmer_cape: v })}
            />
            <Toggle
              label="Ghost Scepter"
              checked={danger.auto_ghost_scepter}
              onChange={(v) => updateDanger({ auto_ghost_scepter: v })}
            />
            <Toggle
              label="Shiva's Guard"
              checked={danger.auto_shivas_guard}
              onChange={(v) => updateDanger({ auto_shivas_guard: v })}
            />
          </Card>

          <Card title="Neutral Items">
            <Toggle
              label="Enable"
              checked={neutral.enabled}
              onChange={(v) => updateNeutral({ enabled: v })}
            />
            <Toggle
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
            <p className="text-xs text-subtle">
              Listed neutrals the runtime has no cast mode for are ignored at
              runtime.
            </p>
          </Card>
        </div>
      </div>
    </div>
  );
}
