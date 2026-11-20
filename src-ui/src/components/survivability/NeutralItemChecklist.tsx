import { useEffect, useState } from "react";
import { X } from "lucide-react";
import type { NeutralItem } from "../../types/config";
import { isTauri } from "../../lib/tauri";
import { mockNeutralItems } from "../../stores/mockData";

const TIERS = [1, 2, 3, 4];

const CAST_MODE_LABELS: Record<NonNullable<NeutralItem["castMode"]>, string> = {
  self_cast: "Self-cast",
  no_target: "No target",
  cursor_targeted: "At cursor",
};

/** Why an item that cannot be ticked is greyed out. */
const BLOCKED_LABELS: Record<Exclude<NeutralItem["support"], "supported">, string> = {
  unsupported: "Not automated",
  inactive: "Retired",
  other_path: "Low-mana path",
  passive: "Passive",
};

/** The pool comes from the backend; browser runs get a fixed slice of it. */
export function useNeutralItems(): NeutralItem[] {
  const [items, setItems] = useState<NeutralItem[]>(isTauri() ? [] : mockNeutralItems);

  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;
    void (async () => {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const result = await invoke<NeutralItem[]>("get_neutral_items");
        if (!cancelled && Array.isArray(result)) setItems(result);
      } catch (e) {
        console.error("Failed to load the neutral item pool:", e);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return items;
}

interface NeutralItemChecklistProps {
  pool: NeutralItem[];
  allowed: string[];
  onChange: (allowed: string[]) => void;
}

/**
 * The danger allowlist as a checklist of the current pool, by tier.
 *
 * Only supported items can be ticked. An item that is ticked but no longer
 * supported, or no longer in the pool at all, stays visible so it can be removed
 * rather than lingering unseen in the config.
 */
export function NeutralItemChecklist({ pool, allowed, onChange }: NeutralItemChecklistProps) {
  const toggle = (name: string, on: boolean) => {
    onChange(on ? [...allowed, name] : allowed.filter((item) => item !== name));
  };
  const inPool = new Set(pool.map((item) => item.name));
  const strays = allowed.filter((name) => !inPool.has(name));

  if (pool.length === 0) {
    return <p className="text-xs text-muted">Loading the neutral item pool…</p>;
  }

  return (
    <div className="space-y-3">
      <label className="text-xs text-subtle">Allowed Items</label>
      {TIERS.map((tier) => {
        const items = pool.filter((item) => item.tier === tier);
        const actives = items.filter((item) => item.support !== "passive");
        const passives = items.length - actives.length;
        if (items.length === 0) return null;
        return (
          <div key={tier} className="space-y-1">
            <p className="text-xs font-medium text-muted">Tier {tier}</p>
            {actives.map((item) => {
              const checked = allowed.includes(item.name);
              const supported = item.support === "supported";
              return (
                <label
                  key={item.name}
                  className={`flex items-center gap-2 text-sm ${
                    supported || checked ? "cursor-pointer text-content" : "cursor-not-allowed text-muted"
                  }`}
                >
                  <input
                    type="checkbox"
                    checked={checked}
                    // A stale tick can always be cleared; a new one needs support.
                    disabled={!supported && !checked}
                    onChange={(e) => toggle(item.name, e.target.checked)}
                    className="accent-gold"
                  />
                  <span className="flex-1">{item.displayName}</span>
                  <span className="text-xs text-subtle">
                    {supported && item.castMode
                      ? CAST_MODE_LABELS[item.castMode]
                      : item.support !== "supported" && BLOCKED_LABELS[item.support]}
                  </span>
                </label>
              );
            })}
            {passives > 0 && (
              <p className="text-xs text-muted">
                + {passives} passive{passives === 1 ? "" : "s"}, nothing to cast
              </p>
            )}
          </div>
        );
      })}
      {strays.length > 0 && (
        <div className="space-y-1">
          <p className="text-xs font-medium text-muted">Not in the current pool</p>
          <div className="flex flex-wrap gap-2">
            {strays.map((name) => (
              <span
                key={name}
                className="inline-flex items-center gap-1 rounded-full border border-border bg-elevated px-2.5 py-0.5 text-xs text-content"
              >
                {name}
                <button
                  type="button"
                  onClick={() => toggle(name, false)}
                  aria-label={`remove ${name}`}
                  className="ml-0.5 rounded-full p-0.5 text-muted hover:text-danger"
                >
                  <X className="h-3 w-3" />
                </button>
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
