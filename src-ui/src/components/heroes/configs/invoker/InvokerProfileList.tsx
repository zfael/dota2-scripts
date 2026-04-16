import { Button } from "../../../common/Button";
import type { InvokerProfile } from "../../../../types/config";
import {
  INVOKER_PRESET_PROFILES,
  getInvokerStepLabel,
} from "./invokerCatalog";

interface InvokerProfileListProps {
  profiles: InvokerProfile[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onDuplicate: (id: string) => void;
  onDelete: (id: string) => void;
  onAddPreset: (presetId: string) => void;
}

function stepSummary(profile: InvokerProfile) {
  if (!profile.steps.length) {
    return "No steps yet";
  }

  return profile.steps
    .map((step) => getInvokerStepLabel(step.target))
    .join(" → ");
}

export function InvokerProfileList({
  profiles,
  selectedId,
  onSelect,
  onDuplicate,
  onDelete,
  onAddPreset,
}: InvokerProfileListProps) {
  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <div className="text-xs font-semibold uppercase tracking-wide text-subtle">
          Preset Library
        </div>
        <div className="grid gap-2 md:grid-cols-2">
          {INVOKER_PRESET_PROFILES.map((preset) => (
            <button
              key={preset.id}
              type="button"
              onClick={() => onAddPreset(preset.id)}
              className="rounded-lg border border-dashed border-border bg-elevated p-3 text-left transition hover:border-border-accent"
            >
              <div className="text-sm font-medium text-content">{preset.name}</div>
              <div className="mt-1 text-xs text-subtle">
                {preset.mode} · {stepSummary(preset)}
              </div>
            </button>
          ))}
        </div>
      </div>

      <div className="space-y-2">
        <div className="text-xs font-semibold uppercase tracking-wide text-subtle">
          Configured Profiles
        </div>
        <div className="space-y-2">
          {profiles.map((profile) => (
            <div
              key={profile.id}
              className={`rounded-lg border p-3 ${
                selectedId === profile.id
                  ? "border-border-accent bg-elevated"
                  : "border-border bg-surface"
              }`}
            >
              <button
                type="button"
                onClick={() => onSelect(profile.id)}
                className="w-full text-left"
              >
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <div className="text-sm font-medium text-content">
                      {profile.name}
                    </div>
                    <div className="text-xs text-subtle">
                      {profile.mode} · {profile.hotkey || "Unbound"} ·{" "}
                      {profile.enabled ? "enabled" : "disabled"}
                    </div>
                  </div>
                  <div className="text-xs uppercase tracking-wide text-subtle">
                    {profile.build_tag || "general"}
                  </div>
                </div>
                <div className="mt-2 text-xs text-subtle">
                  {stepSummary(profile)}
                </div>
              </button>

              <div className="mt-3 flex gap-2">
                <Button
                  variant="secondary"
                  className="px-3"
                  aria-label={`Duplicate ${profile.name}`}
                  onClick={() => onDuplicate(profile.id)}
                >
                  Duplicate
                </Button>
                <Button
                  variant="danger"
                  className="px-3"
                  aria-label={`Delete ${profile.name}`}
                  onClick={() => onDelete(profile.id)}
                >
                  Delete
                </Button>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

