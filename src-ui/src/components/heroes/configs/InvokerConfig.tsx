import { Card } from "../../common/Card";
import { KeyInput } from "../../common/KeyInput";
import { useConfigStore } from "../../../stores/configStore";

export default function InvokerConfig() {
  const config = useConfigStore((s) => s.config.heroes.invoker);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("invoker", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Keybindings">
          <KeyInput label="Standalone Combo Key" value={config.standalone_key} onChange={(v) => set({ standalone_key: v })} />
          <KeyInput label="Panic Key" value={config.panic_key} onChange={(v) => set({ panic_key: v })} />
          <KeyInput label="Prep Key" value={config.prep_key} onChange={(v) => set({ prep_key: v })} />
        </Card>

        <Card title="Orb Keys">
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <label className="w-24 text-xs text-content">Quas:</label>
              <input
                type="text"
                maxLength={1}
                value={config.quas_key}
                onChange={(e) => set({ quas_key: e.target.value })}
                className="w-12 rounded bg-elevated px-2 py-1 text-center text-xs"
              />
            </div>
            <div className="flex items-center gap-2">
              <label className="w-24 text-xs text-content">Wex:</label>
              <input
                type="text"
                maxLength={1}
                value={config.wex_key}
                onChange={(e) => set({ wex_key: e.target.value })}
                className="w-12 rounded bg-elevated px-2 py-1 text-center text-xs"
              />
            </div>
            <div className="flex items-center gap-2">
              <label className="w-24 text-xs text-content">Exort:</label>
              <input
                type="text"
                maxLength={1}
                value={config.exort_key}
                onChange={(e) => set({ exort_key: e.target.value })}
                className="w-12 rounded bg-elevated px-2 py-1 text-center text-xs"
              />
            </div>
            <div className="flex items-center gap-2">
              <label className="w-24 text-xs text-content">Invoke:</label>
              <input
                type="text"
                maxLength={1}
                value={config.invoke_key}
                onChange={(e) => set({ invoke_key: e.target.value })}
                className="w-12 rounded bg-elevated px-2 py-1 text-center text-xs"
              />
            </div>
          </div>
        </Card>

        <Card title="Spell Slots">
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <label className="w-24 text-xs text-content">Primary:</label>
              <input
                type="text"
                maxLength={1}
                value={config.spell_slot_primary_key}
                onChange={(e) => set({ spell_slot_primary_key: e.target.value })}
                className="w-12 rounded bg-elevated px-2 py-1 text-center text-xs"
              />
            </div>
            <div className="flex items-center gap-2">
              <label className="w-24 text-xs text-content">Secondary:</label>
              <input
                type="text"
                maxLength={1}
                value={config.spell_slot_secondary_key}
                onChange={(e) => set({ spell_slot_secondary_key: e.target.value })}
                className="w-12 rounded bg-elevated px-2 py-1 text-center text-xs"
              />
            </div>
          </div>
        </Card>

        <Card title="Combo Profiles">
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <label className="w-32 text-xs text-content">Primary Profile:</label>
              <input
                type="text"
                value={config.primary_profile}
                onChange={(e) => set({ primary_profile: e.target.value })}
                className="flex-1 rounded bg-elevated px-2 py-1 text-xs"
              />
            </div>
            <div className="flex items-center gap-2">
              <label className="w-32 text-xs text-content">Prep Profile:</label>
              <input
                type="text"
                value={config.prep_profile}
                onChange={(e) => set({ prep_profile: e.target.value })}
                className="flex-1 rounded bg-elevated px-2 py-1 text-xs"
              />
            </div>
          </div>
        </Card>

        <Card title="Combo Timings (ms)">
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <label className="w-40 text-xs text-content">Tornado → EMP Delay:</label>
              <input
                type="number"
                value={config.tornado_emp_delay_ms}
                onChange={(e) => set({ tornado_emp_delay_ms: parseInt(e.target.value) || 0 })}
                className="w-20 rounded bg-elevated px-2 py-1 text-xs"
              />
            </div>
            <div className="flex items-center gap-2">
              <label className="w-40 text-xs text-content">Sun Strike Delay:</label>
              <input
                type="number"
                value={config.sun_strike_delay_ms}
                onChange={(e) => set({ sun_strike_delay_ms: parseInt(e.target.value) || 0 })}
                className="w-20 rounded bg-elevated px-2 py-1 text-xs"
              />
            </div>
            <div className="flex items-center gap-2">
              <label className="w-40 text-xs text-content">Meteor → Blast Delay:</label>
              <input
                type="number"
                value={config.meteor_blast_delay_ms}
                onChange={(e) => set({ meteor_blast_delay_ms: parseInt(e.target.value) || 0 })}
                className="w-20 rounded bg-elevated px-2 py-1 text-xs"
              />
            </div>
          </div>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Combo Items" collapsible>
          <p className="text-xs text-muted">
            Configure combo items for Invoker automation. Default items: Spirit Vessel, Rod of Atos.
          </p>
        </Card>

        <Card title="Armlet Override" collapsible>
          <p className="text-xs text-muted">
            Configure armlet override thresholds on the Armlet page.
          </p>
        </Card>
      </div>
    </>
  );
}
