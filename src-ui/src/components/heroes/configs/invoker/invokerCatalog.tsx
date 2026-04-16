import type { ReactNode } from "react";
import type {
  InvokerProfile,
  InvokerProfileStep,
  InvokerProfileStepKind,
} from "../../../../types/config";

export interface InvokerCatalogEntry {
  id: string;
  label: string;
  kind: InvokerProfileStepKind;
  icon: ReactNode;
}

function chip(label: string, classes: string) {
  return (
    <span
      className={`inline-flex h-6 min-w-6 items-center justify-center rounded-md px-1.5 text-[10px] font-semibold text-white ${classes}`}
    >
      {label}
    </span>
  );
}

export const INVOKER_SPELLS: InvokerCatalogEntry[] = [
  { id: "invoker_tornado", label: "Tornado", kind: "spell", icon: chip("TO", "bg-sky-600") },
  { id: "invoker_emp", label: "EMP", kind: "spell", icon: chip("EM", "bg-violet-600") },
  { id: "invoker_sun_strike", label: "Sun Strike", kind: "spell", icon: chip("SS", "bg-amber-600") },
  { id: "invoker_chaos_meteor", label: "Chaos Meteor", kind: "spell", icon: chip("CM", "bg-orange-600") },
  { id: "invoker_deafening_blast", label: "Deafening Blast", kind: "spell", icon: chip("DB", "bg-rose-700") },
  { id: "invoker_ghost_walk", label: "Ghost Walk", kind: "spell", icon: chip("GW", "bg-cyan-700") },
  { id: "invoker_cold_snap", label: "Cold Snap", kind: "spell", icon: chip("CS", "bg-blue-700") },
  { id: "invoker_forge_spirit", label: "Forge Spirit", kind: "spell", icon: chip("FS", "bg-red-700") },
  { id: "invoker_ice_wall", label: "Ice Wall", kind: "spell", icon: chip("IW", "bg-slate-600") },
];

export const INVOKER_ITEMS: InvokerCatalogEntry[] = [
  { id: "item_spirit_vessel", label: "Spirit Vessel", kind: "item", icon: chip("SV", "bg-emerald-700") },
  { id: "item_rod_of_atos", label: "Rod of Atos", kind: "item", icon: chip("AT", "bg-lime-700") },
  { id: "item_sheepstick", label: "Hex", kind: "item", icon: chip("HX", "bg-fuchsia-700") },
  { id: "item_bloodthorn", label: "Bloodthorn", kind: "item", icon: chip("BT", "bg-red-800") },
  { id: "item_orchid", label: "Orchid", kind: "item", icon: chip("OR", "bg-purple-700") },
  { id: "item_black_king_bar", label: "BKB", kind: "item", icon: chip("BK", "bg-yellow-700") },
];

export const INVOKER_LIBRARY = [...INVOKER_SPELLS, ...INVOKER_ITEMS];

export const INVOKER_PRESET_PROFILES: InvokerProfile[] = [
  {
    id: "qw-pickoff",
    name: "QW Pickoff",
    enabled: true,
    hotkey: "Home",
    mode: "combo",
    build_tag: "qw",
    steps: [
      { kind: "item", target: "item_spirit_vessel", delay_after_ms: 50, completion_mode: "fixed_delay", completion_timeout_ms: 3000, notes: "" },
      { kind: "item", target: "item_rod_of_atos", delay_after_ms: 50, completion_mode: "fixed_delay", completion_timeout_ms: 3000, notes: "" },
      { kind: "spell", target: "invoker_tornado", delay_after_ms: 700, completion_mode: "fixed_delay", completion_timeout_ms: 3000, notes: "" },
      { kind: "spell", target: "invoker_emp", delay_after_ms: 100, completion_mode: "fixed_delay", completion_timeout_ms: 3000, notes: "" },
    ],
  },
  {
    id: "qe-burst",
    name: "QE Burst",
    enabled: false,
    hotkey: "PageDown",
    mode: "combo",
    build_tag: "qe",
    steps: [
      { kind: "spell", target: "invoker_sun_strike", delay_after_ms: 150, completion_mode: "wait_for_cooldown", completion_timeout_ms: 3000, notes: "" },
      { kind: "spell", target: "invoker_chaos_meteor", delay_after_ms: 450, completion_mode: "fixed_delay", completion_timeout_ms: 3000, notes: "" },
      { kind: "spell", target: "invoker_deafening_blast", delay_after_ms: 100, completion_mode: "fixed_delay", completion_timeout_ms: 3000, notes: "" },
    ],
  },
  {
    id: "ghost-walk-panic",
    name: "Ghost Walk Panic",
    enabled: true,
    hotkey: "End",
    mode: "combo",
    build_tag: "general",
    steps: [{ kind: "spell", target: "invoker_ghost_walk", delay_after_ms: 100, completion_mode: "fixed_delay", completion_timeout_ms: 3000, notes: "" }],
  },
  {
    id: "meteor-blast-prep",
    name: "Meteor + Blast Prep",
    enabled: true,
    hotkey: "PageUp",
    mode: "prep",
    build_tag: "qe",
    steps: [
      { kind: "spell", target: "invoker_chaos_meteor", delay_after_ms: 0, completion_mode: "fixed_delay", completion_timeout_ms: 3000, notes: "" },
      { kind: "spell", target: "invoker_deafening_blast", delay_after_ms: 0, completion_mode: "fixed_delay", completion_timeout_ms: 3000, notes: "" },
    ],
  },
];

export function getInvokerCatalogEntry(id: string) {
  return INVOKER_LIBRARY.find((entry) => entry.id === id);
}

export function getInvokerStepLabel(target: string) {
  return getInvokerCatalogEntry(target)?.label ?? target;
}

export function createInvokerStep(kind: InvokerProfileStepKind): InvokerProfileStep {
  const fallback = (kind === "spell" ? INVOKER_SPELLS[0] : INVOKER_ITEMS[0])!;
  return {
    kind,
    target: fallback.id,
    delay_after_ms: kind === "spell" ? 100 : 50,
    completion_mode: "fixed_delay",
    completion_timeout_ms: 3000,
    notes: "",
  };
}

