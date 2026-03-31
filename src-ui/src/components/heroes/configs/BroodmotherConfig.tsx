import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { KeyInput } from "../../common/KeyInput";
import { TagList } from "../../common/TagList";
import { useConfigStore } from "../../../stores/configStore";

export default function BroodmotherConfig() {
  const config = useConfigStore((s) => s.config.heroes.broodmother);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("broodmother", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Spider Micro">
          <Toggle label="Enable Spider Micro" checked={config.spider_micro_enabled} onChange={(v) => set({ spider_micro_enabled: v })} />
          <KeyInput label="Spider Control Group Key" value={config.spider_control_group_key} onChange={(v) => set({ spider_control_group_key: v })} />
          <KeyInput label="Reselect Hero Key" value={config.reselect_hero_key} onChange={(v) => set({ reselect_hero_key: v })} />
          <KeyInput label="Standalone Key" value={config.standalone_key} onChange={(v) => set({ standalone_key: v })} />
        </Card>

        <Card title="Auto-Items (Space+Right-Click)">
          <Toggle label="Enable Auto Items" checked={config.auto_items_enabled} onChange={(v) => set({ auto_items_enabled: v })} />
          <TagList label="Item List" items={config.auto_items} onChange={(v) => set({ auto_items: v })} />
          <Toggle label="Auto Abilities First" checked={config.auto_abilities_first} onChange={(v) => set({ auto_abilities_first: v })} />
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Auto Abilities">
          <div className="space-y-2">
            {config.auto_abilities.map((ability, i) => (
              <div key={i} className="flex items-center gap-3 rounded-md border border-border bg-base p-2">
                <span className="text-xs text-muted">#{ability.index}</span>
                <span className="font-mono text-sm text-content">{ability.key.toUpperCase()}</span>
                {ability.hp_threshold != null && (
                  <span className="text-xs text-subtle">HP &lt; {ability.hp_threshold}%</span>
                )}
              </div>
            ))}
          </div>
        </Card>
      </div>
    </>
  );
}

