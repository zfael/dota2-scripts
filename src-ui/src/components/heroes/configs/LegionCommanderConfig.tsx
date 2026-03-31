import { Card } from "../../common/Card";
import { KeyInput } from "../../common/KeyInput";
import { useConfigStore } from "../../../stores/configStore";

export default function LegionCommanderConfig() {
  const config = useConfigStore((s) => s.config.heroes.legion_commander);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("legion_commander", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Keybindings">
          <KeyInput label="Standalone Combo Key" value={config.standalone_key} onChange={(v) => set({ standalone_key: v })} />
        </Card>

        <Card title="Combo Sequence">
          <div className="space-y-1 text-xs text-subtle">
            <p className="font-medium text-content">Combo Order:</p>
            <div className="flex flex-wrap gap-1">
              {["W (Press The Attack)", "Blade Mail", "Mjollnir", "BKB", "Blink", "Orchid/Bloodthorn", "R (Duel)", "Q (Overwhelming Odds)"].map((step, i) => (
                <span key={i} className="rounded bg-elevated px-2 py-0.5 font-mono">
                  {i > 0 && "→ "}{step}
                </span>
              ))}
            </div>
            <p className="mt-2 text-muted">Soul Ring is automatically used before Duel if available.</p>
          </div>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Armlet Override" collapsible>
          <p className="text-xs text-muted">
            Configure armlet override thresholds on the Armlet page.
          </p>
        </Card>
      </div>
    </>
  );
}

