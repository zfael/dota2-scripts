# Invoker Header Profile Display Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show the active Invoker combo profile in the top status header when the current in-game hero is Invoker, with a `Profile: None` fallback when no valid active combo resolves.

**Architecture:** Keep the lookup in `src-ui/src/App.tsx` so the header remains mostly presentational. Add one optional `invokerProfileLabel` prop to `StatusHeader`, resolve it from existing UI/config store state, and cover the behavior with focused Vitest tests.

**Tech Stack:** React 19, TypeScript, Zustand, Vitest, Testing Library

---

## File Map

- Modify: `src-ui/src/App.tsx`
  - compose the optional Invoker profile label from current hero, active combo ID, and Invoker profiles
- Modify: `src-ui/src/components/layout/StatusHeader.tsx`
  - render the optional profile chip next to hero name and level
- Modify: `src-ui/src/components/layout/StatusHeader.test.tsx`
  - cover chip rendering and non-rendering behavior
- Create: `src-ui/src/App.test.tsx`
  - cover the label resolution helper used by `App.tsx`

### Task 1: Resolve the header label in `App.tsx`

**Files:**
- Modify: `src-ui/src/App.tsx`
- Create: `src-ui/src/App.test.tsx`
- Test: `src-ui/src/App.test.tsx`

- [ ] **Step 1: Write the failing App-level label resolution tests**

```tsx
import { describe, expect, it } from "vitest";
import type { InvokerProfile } from "./types/config";
import { getInvokerHeaderProfileLabel } from "./App";

const profiles: InvokerProfile[] = [
  {
    id: "qw-pickoff",
    name: "QW Pickoff",
    enabled: true,
    hotkey: "Home",
    mode: "combo",
    build_tag: "qw",
    steps: [],
  },
  {
    id: "meteor-blast-prep",
    name: "Meteor + Blast Prep",
    enabled: true,
    hotkey: "PageUp",
    mode: "prep",
    build_tag: "qe",
    steps: [],
  },
];

describe("getInvokerHeaderProfileLabel", () => {
  it("returns the active combo name for Invoker", () => {
    expect(
      getInvokerHeaderProfileLabel("Invoker", "qw-pickoff", profiles),
    ).toBe("Profile: QW Pickoff");
  });

  it("returns Profile: None when the active id is missing, disabled, or prep-only", () => {
    expect(
      getInvokerHeaderProfileLabel("Invoker", "missing-profile", profiles),
    ).toBe("Profile: None");
  });

  it("returns undefined for non-Invoker heroes and idle state", () => {
    expect(
      getInvokerHeaderProfileLabel("Shadow Fiend", "qw-pickoff", profiles),
    ).toBeUndefined();
    expect(getInvokerHeaderProfileLabel(undefined, "qw-pickoff", profiles)).toBeUndefined();
  });
});
```

- [ ] **Step 2: Run the App-level test to verify it fails**

Run: `npm --prefix src-ui test -- --run src/App.test.tsx`

Expected: FAIL because `getInvokerHeaderProfileLabel` does not exist yet.

- [ ] **Step 3: Write the minimal App-level implementation**

```tsx
export function getInvokerHeaderProfileLabel(
  heroName: string | undefined,
  activeComboId: string | null,
  profiles: InvokerProfile[],
) {
  if (heroName !== "Invoker") {
    return undefined;
  }

  const activeProfile = profiles.find(
    (profile) =>
      profile.id === activeComboId &&
      profile.mode === "combo" &&
      profile.enabled,
  );

  return `Profile: ${activeProfile?.name ?? "None"}`;
}
```

Then wire it into the existing header props:

```tsx
const invokerProfiles = useConfigStore((s) => s.config.heroes.invoker.profiles);
const activeComboId = useUIStore((s) => s.invokerActiveComboProfileId);
const invokerProfileLabel = getInvokerHeaderProfileLabel(
  game.heroName ?? undefined,
  activeComboId,
  invokerProfiles,
);

<StatusHeader
  heroName={game.heroName ?? undefined}
  heroLevel={game.heroLevel}
  invokerProfileLabel={invokerProfileLabel}
  ...
/>
```

- [ ] **Step 4: Run the App-level test to verify it passes**

Run: `npm --prefix src-ui test -- --run src/App.test.tsx`

Expected: PASS with 3 passing tests.

- [ ] **Step 5: Commit the App-level lookup**

```bash
git add src-ui/src/App.tsx src-ui/src/App.test.tsx
git commit -m "feat(ui): resolve invoker header profile label"
```

### Task 2: Render the profile chip in `StatusHeader`

**Files:**
- Modify: `src-ui/src/components/layout/StatusHeader.tsx`
- Modify: `src-ui/src/components/layout/StatusHeader.test.tsx`
- Test: `src-ui/src/components/layout/StatusHeader.test.tsx`

- [ ] **Step 1: Write the failing header rendering tests**

Add these cases to `StatusHeader.test.tsx`:

```tsx
it("renders the Invoker profile chip when provided", () => {
  render(
    <StatusHeader
      heroName="Invoker"
      heroLevel={30}
      hpPercent={100}
      manaPercent={100}
      connected={true}
      invokerProfileLabel="Profile: QW Pickoff"
      {...defaultProps}
    />,
  );

  expect(screen.getByText("Profile: QW Pickoff")).toBeInTheDocument();
});

it("does not render a profile chip when none is provided", () => {
  render(
    <StatusHeader
      heroName="Invoker"
      heroLevel={30}
      hpPercent={100}
      manaPercent={100}
      connected={true}
      {...defaultProps}
    />,
  );

  expect(screen.queryByText(/Profile:/i)).not.toBeInTheDocument();
});
```

- [ ] **Step 2: Run the header test to verify it fails**

Run: `npm --prefix src-ui test -- --run src/components/layout/StatusHeader.test.tsx`

Expected: FAIL because `StatusHeader` does not accept or render `invokerProfileLabel`.

- [ ] **Step 3: Write the minimal header implementation**

Extend the prop type and render the chip in the hero identity row:

```tsx
interface StatusHeaderProps {
  heroName?: string;
  heroLevel?: number;
  invokerProfileLabel?: string;
  ...
}
```

```tsx
<div className="flex items-center gap-2">
  <span className="font-semibold text-content">{heroName}</span>
  <span className="rounded bg-elevated px-1.5 py-0.5 font-mono text-xs text-subtle">
    Lv. {heroLevel}
  </span>
  {invokerProfileLabel && (
    <span
      className="max-w-48 truncate rounded bg-elevated px-2 py-0.5 text-xs text-subtle"
      title={invokerProfileLabel}
    >
      {invokerProfileLabel}
    </span>
  )}
</div>
```

- [ ] **Step 4: Run the header test to verify it passes**

Run: `npm --prefix src-ui test -- --run src/components/layout/StatusHeader.test.tsx`

Expected: PASS with the existing tests plus the new profile-chip assertions.

- [ ] **Step 5: Commit the header rendering change**

```bash
git add src-ui/src/components/layout/StatusHeader.tsx src-ui/src/components/layout/StatusHeader.test.tsx
git commit -m "feat(ui): show invoker profile in status header"
```

### Task 3: Run focused and full UI verification

**Files:**
- Test: `src-ui/src/App.test.tsx`
- Test: `src-ui/src/components/layout/StatusHeader.test.tsx`

- [ ] **Step 1: Run the focused UI tests together**

Run: `npm --prefix src-ui test -- --run src/App.test.tsx src/components/layout/StatusHeader.test.tsx`

Expected: PASS with all new header-profile tests green.

- [ ] **Step 2: Run the full React UI suite**

Run: `npm --prefix src-ui test`

Expected: PASS for the complete Vitest suite.

- [ ] **Step 3: Run the repo verification commands**

Run:

```bash
cargo test
npm --prefix src-ui test
cargo build --release
```

Expected:

- `cargo test` passes
- `npm --prefix src-ui test` passes
- `cargo build --release` passes

- [ ] **Step 4: Commit any final polish if needed**

```bash
git add src-ui/src/App.tsx src-ui/src/App.test.tsx src-ui/src/components/layout/StatusHeader.tsx src-ui/src/components/layout/StatusHeader.test.tsx
git commit -m "test: cover invoker header profile display"
```
