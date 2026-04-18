import { useEffect, useMemo, useState } from "react";
import { Button } from "../../common/Button";
import { Card } from "../../common/Card";
import { KeyInput } from "../../common/KeyInput";
import { useConfigStore } from "../../../stores/configStore";
import { useUIStore } from "../../../stores/uiStore";
import type { InvokerProfile } from "../../../types/config";
import { InvokerProfileEditor } from "./invoker/InvokerProfileEditor";
import { InvokerProfileList } from "./invoker/InvokerProfileList";
import {
  INVOKER_PRESET_PROFILES,
  createInvokerStep,
} from "./invoker/invokerCatalog";

function slugify(value: string) {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function nextProfileId(baseName: string, profiles: InvokerProfile[]) {
  const base = slugify(baseName) || "invoker-profile";
  let candidate = base;
  let counter = 2;

  while (profiles.some((profile) => profile.id === candidate)) {
    candidate = `${base}-${counter}`;
    counter += 1;
  }

  return candidate;
}

function cloneProfile(profile: InvokerProfile, profiles: InvokerProfile[]) {
  const copyName = `${profile.name} Copy`;
  return {
    ...profile,
    id: nextProfileId(copyName, profiles),
    name: copyName,
    steps: profile.steps.map((step) => ({ ...step })),
  };
}

function createBlankProfile(
  mode: InvokerProfile["mode"],
  profiles: InvokerProfile[],
): InvokerProfile {
  const name = mode === "combo" ? "Custom Combo" : "Custom Prep";
  return {
    id: nextProfileId(name, profiles),
    name,
    enabled: true,
    hotkey: "",
    mode,
    execution_style: "automatic",
    build_tag: "general",
    steps: [createInvokerStep("spell")],
  };
}

export default function InvokerConfig() {
  const config = useConfigStore((state) => state.config.heroes.invoker);
  const update = useConfigStore((state) => state.updateHeroConfig);
  const activeComboId = useUIStore((state) => state.invokerActiveComboProfileId);
  const setActiveComboId = useUIStore(
    (state) => state.setInvokerActiveComboProfileId,
  );
  const set = (updates: Partial<typeof config>) => update("invoker", updates);
  const [selectedId, setSelectedId] = useState<string | null>(
    config.profiles[0]?.id ?? null,
  );

  const selected = useMemo(
    () =>
      config.profiles.find((profile) => profile.id === selectedId) ??
      config.profiles[0] ??
      null,
    [config.profiles, selectedId],
  );
  const activeComboName = useMemo(
    () =>
      config.profiles.find(
        (profile) =>
          profile.id === activeComboId &&
          profile.mode === "combo" &&
          profile.enabled,
      )?.name ?? null,
    [activeComboId, config.profiles],
  );

  useEffect(() => {
    if (!activeComboId) {
      return;
    }

    const activeProfile = config.profiles.find(
      (profile) => profile.id === activeComboId,
    );

    if (
      !activeProfile ||
      activeProfile.mode !== "combo" ||
      !activeProfile.enabled
    ) {
      setActiveComboId(null);
    }
  }, [activeComboId, config.profiles, setActiveComboId]);

  const setProfiles = (profiles: InvokerProfile[]) => set({ profiles });

  const handleSelectProfile = (id: string) => {
    setSelectedId(id);

    const profile = config.profiles.find((candidate) => candidate.id === id);
    if (profile?.mode === "combo" && profile.enabled) {
      setActiveComboId(profile.id);
    }
  };

  return (
    <>
      <div className="space-y-4">
        <Card title="Core Keys">
          <div className="grid gap-3 md:grid-cols-2">
            <KeyInput
              label="Quas"
              value={config.quas_key}
              onChange={(quas_key) => set({ quas_key })}
            />
            <KeyInput
              label="Wex"
              value={config.wex_key}
              onChange={(wex_key) => set({ wex_key })}
            />
            <KeyInput
              label="Exort"
              value={config.exort_key}
              onChange={(exort_key) => set({ exort_key })}
            />
            <KeyInput
              label="Invoke"
              value={config.invoke_key}
              onChange={(invoke_key) => set({ invoke_key })}
            />
            <KeyInput
              label="Primary Spell Slot"
              value={config.spell_slot_primary_key}
              onChange={(spell_slot_primary_key) => set({ spell_slot_primary_key })}
            />
            <KeyInput
              label="Secondary Spell Slot"
              value={config.spell_slot_secondary_key}
              onChange={(spell_slot_secondary_key) =>
                set({ spell_slot_secondary_key })
              }
            />
            <KeyInput
              label="Cycle Combo Profiles"
              value={config.cycle_combo_profiles_hotkey}
              onChange={(cycle_combo_profiles_hotkey) =>
                set({ cycle_combo_profiles_hotkey })
              }
            />
          </div>
        </Card>

        <Card title="Profiles">
          <p className="text-sm text-subtle">
            Active combo:{" "}
            <span className="font-medium text-content">
              {activeComboName ?? "None"}
            </span>
          </p>

          <div className="mb-3 flex flex-wrap gap-2">
            <Button
              variant="secondary"
              onClick={() => {
                const profile = createBlankProfile("combo", config.profiles);
                setProfiles([...config.profiles, profile]);
                setSelectedId(profile.id);
              }}
            >
              New Combo Profile
            </Button>
            <Button
              variant="secondary"
              onClick={() => {
                const profile = createBlankProfile("prep", config.profiles);
                setProfiles([...config.profiles, profile]);
                setSelectedId(profile.id);
              }}
            >
              New Prep Profile
            </Button>
          </div>

          <InvokerProfileList
            profiles={config.profiles}
            activeComboId={activeComboId}
            selectedId={selected?.id ?? null}
            onSelect={handleSelectProfile}
            onDuplicate={(id) => {
              const source = config.profiles.find((profile) => profile.id === id);
              if (!source) {
                return;
              }

              const copy = cloneProfile(source, config.profiles);
              setProfiles([...config.profiles, copy]);
              setSelectedId(copy.id);
            }}
            onDelete={(id) => {
              const remaining = config.profiles.filter((profile) => profile.id !== id);
              setProfiles(remaining);
              if (selected?.id === id) {
                setSelectedId(remaining[0]?.id ?? null);
              }
            }}
            onAddPreset={(presetId) => {
              const preset = INVOKER_PRESET_PROFILES.find(
                (profile) => profile.id === presetId,
              );
              if (!preset) {
                return;
              }

              const next = cloneProfile(preset, config.profiles);
              setProfiles([...config.profiles, next]);
              setSelectedId(next.id);
            }}
          />
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Profile Editor">
          {selected ? (
            <InvokerProfileEditor
              profile={selected}
              onChange={(next) =>
                setProfiles(
                  config.profiles.map((profile) =>
                    profile.id === next.id ? next : profile,
                  ),
                )
              }
            />
          ) : (
            <p className="text-sm text-subtle">Select a profile to edit.</p>
          )}
        </Card>

        <Card title="Armlet Override" collapsible>
          <p className="text-xs text-muted">
            Invoker still uses the shared Armlet page for override thresholds.
          </p>
        </Card>
      </div>
    </>
  );
}
