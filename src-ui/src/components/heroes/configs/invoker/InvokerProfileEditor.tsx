import { Button } from "../../../common/Button";
import { Dropdown } from "../../../common/Dropdown";
import { KeyInput } from "../../../common/KeyInput";
import { NumberInput } from "../../../common/NumberInput";
import { Toggle } from "../../../common/Toggle";
import type {
  InvokerProfile,
  InvokerProfileStep,
  InvokerProfileStepKind,
} from "../../../../types/config";
import {
  INVOKER_ITEMS,
  INVOKER_SPELLS,
  createInvokerStep,
  getInvokerCatalogEntry,
  getInvokerStepLabel,
} from "./invokerCatalog";

interface InvokerProfileEditorProps {
  profile: InvokerProfile;
  onChange: (next: InvokerProfile) => void;
}

const MODE_OPTIONS = [
  { value: "combo", label: "Combo" },
  { value: "prep", label: "Prep" },
];

const BUILD_TAG_OPTIONS = [
  { value: "general", label: "General" },
  { value: "qw", label: "QW" },
  { value: "qe", label: "QE" },
];

const KIND_OPTIONS = [
  { value: "spell", label: "Spell" },
  { value: "item", label: "Item" },
];

function cloneSteps(steps: InvokerProfileStep[]) {
  return steps.map((step) => ({ ...step }));
}

function targetOptions(kind: InvokerProfileStepKind) {
  const source = kind === "spell" ? INVOKER_SPELLS : INVOKER_ITEMS;
  return source.map((entry) => ({ value: entry.id, label: entry.label }));
}

export function InvokerProfileEditor({
  profile,
  onChange,
}: InvokerProfileEditorProps) {
  const setStep = (index: number, next: InvokerProfileStep) => {
    const steps = cloneSteps(profile.steps);
    steps[index] = next;
    onChange({ ...profile, steps });
  };

  const moveStep = (index: number, direction: -1 | 1) => {
    const nextIndex = index + direction;
    if (nextIndex < 0 || nextIndex >= profile.steps.length) {
      return;
    }

    const steps = cloneSteps(profile.steps);
    const [step] = steps.splice(index, 1);
    steps.splice(nextIndex, 0, step);
    onChange({ ...profile, steps });
  };

  const removeStep = (index: number) => {
    onChange({
      ...profile,
      steps: profile.steps.filter((_, stepIndex) => stepIndex !== index),
    });
  };

  const addStep = (kind: InvokerProfileStepKind) => {
    onChange({
      ...profile,
      steps: [...profile.steps, createInvokerStep(kind)],
    });
  };

  return (
    <div className="space-y-4">
      <div className="grid gap-3 md:grid-cols-2">
        <div className="space-y-1">
          <label className="text-xs text-subtle">Profile Name</label>
          <input
            value={profile.name}
            onChange={(event) =>
              onChange({ ...profile, name: event.target.value })
            }
            className="h-8 w-full rounded-md border border-border bg-input px-3 text-sm text-content focus:border-border-accent focus:outline-none"
          />
        </div>
        <KeyInput
          label="Hotkey"
          value={profile.hotkey}
          onChange={(hotkey) => onChange({ ...profile, hotkey })}
        />
        <Dropdown
          label="Mode"
          value={profile.mode}
          options={MODE_OPTIONS}
          onChange={(mode) =>
            onChange({ ...profile, mode: mode as InvokerProfile["mode"] })
          }
        />
        <Dropdown
          label="Build Tag"
          value={profile.build_tag || "general"}
          options={BUILD_TAG_OPTIONS}
          onChange={(build_tag) => onChange({ ...profile, build_tag })}
        />
      </div>

      <Toggle
        label="Enable Profile"
        checked={profile.enabled}
        onChange={(enabled) => onChange({ ...profile, enabled })}
      />

      <div className="rounded-lg border border-border bg-elevated p-3">
        <div className="text-xs font-semibold uppercase tracking-wide text-subtle">
          Execution Preview
        </div>
        <div className="mt-2 text-sm text-content">
          {profile.steps.length
            ? profile.steps.map((step) => getInvokerStepLabel(step.target)).join(" → ")
            : "No steps configured"}
        </div>
      </div>

      <div className="space-y-3">
        {profile.steps.map((step, index) => {
          const entry = getInvokerCatalogEntry(step.target);
          const options = targetOptions(step.kind);

          return (
            <div
              key={`${profile.id}-${index}`}
              className="rounded-lg border border-border bg-elevated p-3"
            >
              <div className="flex items-center justify-between gap-3">
                <div className="flex items-center gap-2">
                  {entry?.icon ?? (
                    <span className="inline-flex h-6 min-w-6 items-center justify-center rounded-md bg-input px-1.5 text-[10px] font-semibold text-subtle">
                      {step.kind === "spell" ? "SP" : "IT"}
                    </span>
                  )}
                  <div>
                    <div className="text-sm font-medium text-content">
                      Step {index + 1}: {entry?.label ?? step.target}
                    </div>
                    <div className="text-xs text-subtle">
                      {step.kind} · {step.target}
                    </div>
                  </div>
                </div>

                <div className="flex gap-2">
                  <Button
                    variant="secondary"
                    className="px-2"
                    aria-label={`Move step ${index + 1} up`}
                    onClick={() => moveStep(index, -1)}
                    disabled={index === 0}
                  >
                    ↑
                  </Button>
                  <Button
                    variant="secondary"
                    className="px-2"
                    aria-label={`Move step ${index + 1} down`}
                    onClick={() => moveStep(index, 1)}
                    disabled={index === profile.steps.length - 1}
                  >
                    ↓
                  </Button>
                  <Button
                    variant="danger"
                    className="px-2"
                    aria-label={`Remove step ${index + 1}`}
                    onClick={() => removeStep(index)}
                  >
                    Remove
                  </Button>
                </div>
              </div>

              <div className="mt-3 grid gap-3 md:grid-cols-2">
                <Dropdown
                  label="Kind"
                  value={step.kind}
                  options={KIND_OPTIONS}
                  onChange={(kind) => {
                    const nextKind = kind as InvokerProfileStepKind;
                    const nextTarget = targetOptions(nextKind)[0]?.value ?? step.target;
                    setStep(index, {
                      kind: nextKind,
                      target: nextTarget,
                      delay_after_ms: step.delay_after_ms,
                      notes: step.notes,
                    });
                  }}
                />
                <Dropdown
                  label="Preset Target"
                  value={options.some((option) => option.value === step.target) ? step.target : options[0]?.value ?? ""}
                  options={options}
                  onChange={(target) => setStep(index, { ...step, target })}
                />
                <div className="space-y-1">
                  <label className="text-xs text-subtle">Custom Target ID</label>
                  <input
                    value={step.target}
                    onChange={(event) =>
                      setStep(index, { ...step, target: event.target.value })
                    }
                    className="h-8 w-full rounded-md border border-border bg-input px-3 font-mono text-sm text-content focus:border-border-accent focus:outline-none"
                  />
                </div>
                <NumberInput
                  label="Delay After"
                  value={step.delay_after_ms}
                  onChange={(delay_after_ms) =>
                    setStep(index, { ...step, delay_after_ms })
                  }
                  suffix="ms"
                />
              </div>

              <div className="mt-3 space-y-1">
                <label className="text-xs text-subtle">Notes</label>
                <input
                  value={step.notes}
                  onChange={(event) =>
                    setStep(index, { ...step, notes: event.target.value })
                  }
                  className="h-8 w-full rounded-md border border-border bg-input px-3 text-sm text-content focus:border-border-accent focus:outline-none"
                />
              </div>
            </div>
          );
        })}
      </div>

      <div className="flex flex-wrap gap-2">
        <Button variant="secondary" onClick={() => addStep("spell")}>
          Add Spell Step
        </Button>
        <Button variant="secondary" onClick={() => addStep("item")}>
          Add Item Step
        </Button>
      </div>
    </div>
  );
}

