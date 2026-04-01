# React UI Foundation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a complete React frontend for the Dota 2 Scripts application with all components, layout, stores, routing, and pages — running standalone with mock data, ready for Tauri integration.

**Architecture:** Vite + React 18 + TypeScript app in `src-ui/`. Tailwind CSS v4 for styling with custom design tokens matching the approved Pencil mockups. Zustand stores provide mock data for all game state, config, and activity. React Router v6 handles page navigation. Every component is tested with Vitest + React Testing Library.

**Tech Stack:** React 18, TypeScript, Vite, Tailwind CSS v4, Zustand, React Router v6, Lucide React, Vitest, React Testing Library

**Prerequisite:** Working directory is the worktree at `.worktrees/react-ui-migration/`

**Future plans (not in scope here):**
- Plan 2: Tauri v2 integration — wrapping this React app in Tauri, Rust command wrappers, IPC hooks, real data wiring
- Plan 3: E2E testing + polish

---

## Phase 1: Project Foundation

### Task 1: Scaffold Vite + React + TypeScript Project

**Files:**
- Create: `src-ui/` (entire directory via Vite scaffold)
- Modify: `src-ui/vite.config.ts`
- Modify: `src-ui/tsconfig.json`

- [ ] **Step 1: Create the Vite project**

```bash
cd .worktrees/react-ui-migration
npm create vite@latest src-ui -- --template react-ts
```

- [ ] **Step 2: Install base dependencies**

```bash
cd src-ui
npm install
```

- [ ] **Step 3: Verify the scaffold works**

```bash
npm run build
```

Expected: Build succeeds with no errors.

- [ ] **Step 4: Clean up scaffold boilerplate**

Delete these files:
- `src/App.css`
- `src/index.css`
- `src/assets/react.svg`
- `public/vite.svg`

Replace `src/App.tsx` with:

```tsx
export default function App() {
  return (
    <div className="min-h-screen bg-base text-content">
      <h1 className="text-2xl font-semibold p-8">D2 Scripts</h1>
    </div>
  );
}
```

Replace `src/main.tsx` with:

```tsx
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles/global.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
```

- [ ] **Step 5: Commit**

```bash
git add src-ui/
git commit -m "feat(ui): scaffold Vite + React + TypeScript project"
```

---

### Task 2: Install Dependencies and Configure Build Tools

**Files:**
- Modify: `src-ui/package.json` (via npm install)
- Modify: `src-ui/vite.config.ts`
- Modify: `src-ui/tsconfig.json`
- Create: `src-ui/vitest.setup.ts`

- [ ] **Step 1: Install production dependencies**

```bash
cd src-ui
npm install zustand react-router-dom lucide-react
```

- [ ] **Step 2: Install Tailwind CSS v4 with Vite plugin**

```bash
npm install tailwindcss @tailwindcss/vite
```

- [ ] **Step 3: Install dev dependencies for testing**

```bash
npm install -D vitest @testing-library/react @testing-library/jest-dom @testing-library/user-event jsdom
```

- [ ] **Step 4: Configure Vite with Tailwind and Vitest**

Replace `src-ui/vite.config.ts`:

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: "./vitest.setup.ts",
    css: true,
  },
});
```

- [ ] **Step 5: Create Vitest setup file**

Create `src-ui/vitest.setup.ts`:

```ts
import "@testing-library/jest-dom/vitest";
```

- [ ] **Step 6: Add test types to tsconfig**

In `src-ui/tsconfig.app.json`, add `"vitest/globals"` to `compilerOptions.types`:

```json
{
  "compilerOptions": {
    "types": ["vitest/globals"]
  }
}
```

- [ ] **Step 7: Add test script to package.json**

Add to `src-ui/package.json` scripts:

```json
{
  "scripts": {
    "test": "vitest run",
    "test:watch": "vitest"
  }
}
```

- [ ] **Step 8: Verify build and test work**

```bash
npm run build && npm test
```

Expected: Build succeeds. Tests pass (no tests yet, 0 test files).

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "feat(ui): install deps and configure Tailwind, Vitest"
```

---

### Task 3: Design Tokens and Global Styles

**Files:**
- Create: `src-ui/src/styles/global.css`

- [ ] **Step 1: Create global stylesheet with design tokens**

Create `src-ui/src/styles/global.css`:

```css
@import "tailwindcss";

@theme {
  /* Surfaces */
  --color-base: #0D0F12;
  --color-surface: #161A21;
  --color-elevated: #1E2330;
  --color-input: #12151B;

  /* Borders */
  --color-border: #2A3040;
  --color-border-accent: #C8AA6E;

  /* Text */
  --color-content: #E8E6E3;
  --color-subtle: #8B9BB4;
  --color-muted: #4A5568;

  /* Accents */
  --color-gold: #C8AA6E;
  --color-danger: #E74C3C;
  --color-success: #2ECC71;
  --color-info: #3498DB;
  --color-warning: #F39C12;

  /* Terminal */
  --color-terminal: #00FF88;
  --color-terminal-bg: #0A0E14;

  /* Fonts */
  --font-sans: "Inter", ui-sans-serif, system-ui, sans-serif;
  --font-mono: "JetBrains Mono", ui-monospace, monospace;

  /* Border Radius */
  --radius-sm: 4px;
  --radius-md: 6px;
  --radius-lg: 8px;
}

/* Google Fonts */
@import url("https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;700&display=swap");

/* Base reset */
body {
  @apply bg-base text-content font-sans antialiased;
  margin: 0;
  min-width: 900px;
  min-height: 650px;
}

/* Scrollbar styling */
::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

::-webkit-scrollbar-track {
  background: var(--color-base);
}

::-webkit-scrollbar-thumb {
  background: var(--color-muted);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--color-subtle);
}

/* Focus ring */
*:focus-visible {
  outline: 2px solid var(--color-border-accent);
  outline-offset: 2px;
}
```

- [ ] **Step 2: Verify the app renders with design tokens**

```bash
npm run build
```

Expected: Build succeeds. The app background should be `#0D0F12` (near-black).

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(ui): add design tokens and global styles"
```

---

## Phase 2: Common Components

### Task 4: Toggle Component

**Files:**
- Create: `src-ui/src/components/common/Toggle.tsx`
- Create: `src-ui/src/components/common/Toggle.test.tsx`

- [ ] **Step 1: Write the Toggle test**

Create `src-ui/src/components/common/Toggle.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Toggle } from "./Toggle";

describe("Toggle", () => {
  it("renders with label", () => {
    render(<Toggle label="Enable Feature" checked={false} onChange={() => {}} />);
    expect(screen.getByText("Enable Feature")).toBeInTheDocument();
  });

  it("calls onChange when clicked", async () => {
    const onChange = vi.fn();
    render(<Toggle label="Enable" checked={false} onChange={onChange} />);
    await userEvent.click(screen.getByRole("switch"));
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("renders checked state", () => {
    render(<Toggle label="Active" checked={true} onChange={() => {}} />);
    expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "true");
  });

  it("respects disabled prop", async () => {
    const onChange = vi.fn();
    render(<Toggle label="Disabled" checked={false} onChange={onChange} disabled />);
    await userEvent.click(screen.getByRole("switch"));
    expect(onChange).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd src-ui && npm test -- Toggle
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement the Toggle component**

Create `src-ui/src/components/common/Toggle.tsx`:

```tsx
interface ToggleProps {
  label?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

export function Toggle({ label, checked, onChange, disabled = false }: ToggleProps) {
  return (
    <label className="flex items-center justify-between gap-3 cursor-pointer select-none">
      {label && <span className="text-sm text-subtle">{label}</span>}
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => !disabled && onChange(!checked)}
        className={`
          relative inline-flex h-5 w-9 shrink-0 rounded-full transition-colors duration-200
          ${checked ? "bg-gold" : "bg-input"}
          ${disabled ? "opacity-50 cursor-not-allowed" : "cursor-pointer"}
        `}
      >
        <span
          className={`
            pointer-events-none inline-block h-4 w-4 rounded-full shadow-sm
            transform transition-transform duration-200 mt-0.5
            ${checked ? "translate-x-4 bg-white" : "translate-x-0.5 bg-muted"}
          `}
        />
      </button>
    </label>
  );
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd src-ui && npm test -- Toggle
```

Expected: All 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(ui): add Toggle component"
```

---

### Task 5: Card Component

**Files:**
- Create: `src-ui/src/components/common/Card.tsx`
- Create: `src-ui/src/components/common/Card.test.tsx`

- [ ] **Step 1: Write the Card test**

Create `src-ui/src/components/common/Card.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Card } from "./Card";

describe("Card", () => {
  it("renders title and children", () => {
    render(<Card title="Settings"><p>Content here</p></Card>);
    expect(screen.getByText("Settings")).toBeInTheDocument();
    expect(screen.getByText("Content here")).toBeInTheDocument();
  });

  it("collapses content when collapsible header is clicked", async () => {
    render(
      <Card title="Collapsible" collapsible>
        <p>Hidden content</p>
      </Card>,
    );
    expect(screen.getByText("Hidden content")).toBeVisible();
    await userEvent.click(screen.getByText("Collapsible"));
    expect(screen.queryByText("Hidden content")).not.toBeVisible();
  });

  it("starts collapsed when defaultOpen is false", () => {
    render(
      <Card title="Closed" collapsible defaultOpen={false}>
        <p>Invisible</p>
      </Card>,
    );
    expect(screen.queryByText("Invisible")).not.toBeVisible();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd src-ui && npm test -- Card
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement the Card component**

Create `src-ui/src/components/common/Card.tsx`:

```tsx
import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";

interface CardProps {
  title: string;
  children: React.ReactNode;
  collapsible?: boolean;
  defaultOpen?: boolean;
  className?: string;
}

export function Card({
  title,
  children,
  collapsible = false,
  defaultOpen = true,
  className = "",
}: CardProps) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div
      className={`rounded-lg border border-border bg-surface p-4 ${className}`}
    >
      <button
        type="button"
        onClick={() => collapsible && setOpen(!open)}
        className={`flex w-full items-center justify-between text-left ${
          collapsible ? "cursor-pointer" : "cursor-default"
        }`}
      >
        <h3 className="text-sm font-semibold text-content">{title}</h3>
        {collapsible &&
          (open ? (
            <ChevronDown className="h-4 w-4 text-subtle" />
          ) : (
            <ChevronRight className="h-4 w-4 text-subtle" />
          ))}
      </button>
      <div
        className={`mt-3 space-y-3 ${open ? "" : "hidden"}`}
        aria-hidden={!open}
      >
        {children}
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd src-ui && npm test -- Card
```

Expected: All 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(ui): add Card component"
```

---

### Task 6: Button Components

**Files:**
- Create: `src-ui/src/components/common/Button.tsx`
- Create: `src-ui/src/components/common/Button.test.tsx`

- [ ] **Step 1: Write the Button test**

Create `src-ui/src/components/common/Button.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Button } from "./Button";

describe("Button", () => {
  it("renders primary button by default", () => {
    render(<Button>Save</Button>);
    const btn = screen.getByRole("button", { name: "Save" });
    expect(btn).toBeInTheDocument();
    expect(btn.className).toContain("bg-gold");
  });

  it("renders secondary variant", () => {
    render(<Button variant="secondary">Cancel</Button>);
    const btn = screen.getByRole("button", { name: "Cancel" });
    expect(btn.className).toContain("bg-elevated");
  });

  it("renders danger variant", () => {
    render(<Button variant="danger">Delete</Button>);
    const btn = screen.getByRole("button", { name: "Delete" });
    expect(btn.className).toContain("bg-danger");
  });

  it("calls onClick", async () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Click</Button>);
    await userEvent.click(screen.getByRole("button"));
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("disables button", async () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick} disabled>No</Button>);
    await userEvent.click(screen.getByRole("button"));
    expect(onClick).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd src-ui && npm test -- Button
```

- [ ] **Step 3: Implement the Button component**

Create `src-ui/src/components/common/Button.tsx`:

```tsx
interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "danger";
  children: React.ReactNode;
}

const variants = {
  primary: "bg-gold text-base hover:brightness-110 font-medium",
  secondary: "bg-elevated text-content border border-border hover:bg-surface",
  danger: "bg-danger text-white hover:brightness-110 font-medium",
};

export function Button({
  variant = "primary",
  children,
  className = "",
  disabled,
  ...props
}: ButtonProps) {
  return (
    <button
      type="button"
      disabled={disabled}
      className={`
        inline-flex items-center justify-center gap-2 rounded-md px-4 h-8 text-sm
        transition-all duration-150
        ${variants[variant]}
        ${disabled ? "opacity-50 cursor-not-allowed" : "cursor-pointer"}
        ${className}
      `}
      {...props}
    >
      {children}
    </button>
  );
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd src-ui && npm test -- Button
```

Expected: All 5 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(ui): add Button component"
```

---

### Task 7: Slider Component

**Files:**
- Create: `src-ui/src/components/common/Slider.tsx`
- Create: `src-ui/src/components/common/Slider.test.tsx`

- [ ] **Step 1: Write the Slider test**

Create `src-ui/src/components/common/Slider.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { Slider } from "./Slider";

describe("Slider", () => {
  it("renders with label and value", () => {
    render(
      <Slider label="HP Threshold" value={70} min={0} max={100} onChange={() => {}} suffix="%" />,
    );
    expect(screen.getByText("HP Threshold")).toBeInTheDocument();
    expect(screen.getByText("70%")).toBeInTheDocument();
  });

  it("renders the range input", () => {
    render(<Slider label="Test" value={50} min={0} max={100} onChange={() => {}} />);
    const input = screen.getByRole("slider");
    expect(input).toHaveValue("50");
  });

  it("applies aria attributes", () => {
    render(
      <Slider label="Volume" value={30} min={0} max={100} onChange={() => {}} suffix="%" />,
    );
    const input = screen.getByRole("slider");
    expect(input).toHaveAttribute("aria-valuemin", "0");
    expect(input).toHaveAttribute("aria-valuemax", "100");
    expect(input).toHaveAttribute("aria-valuenow", "30");
    expect(input).toHaveAttribute("aria-valuetext", "30%");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd src-ui && npm test -- Slider
```

- [ ] **Step 3: Implement the Slider component**

Create `src-ui/src/components/common/Slider.tsx`:

```tsx
interface SliderProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
  suffix?: string;
  disabled?: boolean;
}

export function Slider({
  label,
  value,
  min,
  max,
  step = 1,
  onChange,
  suffix = "",
  disabled = false,
}: SliderProps) {
  const pct = ((value - min) / (max - min)) * 100;

  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between">
        <label className="text-xs text-subtle">{label}</label>
        <span className="font-mono text-xs text-gold">
          {value}
          {suffix}
        </span>
      </div>
      <div className="relative h-1.5 w-full rounded-full bg-input">
        <div
          className="absolute left-0 top-0 h-full rounded-full bg-gold"
          style={{ width: `${pct}%` }}
        />
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
          disabled={disabled}
          aria-valuemin={min}
          aria-valuemax={max}
          aria-valuenow={value}
          aria-valuetext={`${value}${suffix}`}
          className="absolute inset-0 w-full cursor-pointer opacity-0"
        />
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd src-ui && npm test -- Slider
```

Expected: All 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(ui): add Slider component"
```

---

### Task 8: NumberInput and KeyInput Components

**Files:**
- Create: `src-ui/src/components/common/NumberInput.tsx`
- Create: `src-ui/src/components/common/KeyInput.tsx`
- Create: `src-ui/src/components/common/NumberInput.test.tsx`

- [ ] **Step 1: Write the NumberInput test**

Create `src-ui/src/components/common/NumberInput.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { NumberInput } from "./NumberInput";

describe("NumberInput", () => {
  it("renders with label and value", () => {
    render(<NumberInput label="Port" value={3000} onChange={() => {}} />);
    expect(screen.getByText("Port")).toBeInTheDocument();
    expect(screen.getByDisplayValue("3000")).toBeInTheDocument();
  });

  it("renders suffix", () => {
    render(<NumberInput label="Delay" value={100} onChange={() => {}} suffix="ms" />);
    expect(screen.getByText("ms")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Implement NumberInput**

Create `src-ui/src/components/common/NumberInput.tsx`:

```tsx
interface NumberInputProps {
  label: string;
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  suffix?: string;
  disabled?: boolean;
}

export function NumberInput({
  label,
  value,
  onChange,
  min,
  max,
  suffix,
  disabled = false,
}: NumberInputProps) {
  return (
    <div className="space-y-1">
      <label className="text-xs text-subtle">{label}</label>
      <div className="flex items-center gap-2">
        <input
          type="number"
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
          min={min}
          max={max}
          disabled={disabled}
          className="h-8 w-full rounded-md border border-border bg-input px-3 font-mono text-sm
                     text-content focus:border-border-accent focus:outline-none
                     disabled:cursor-not-allowed disabled:opacity-50"
        />
        {suffix && <span className="text-xs text-subtle shrink-0">{suffix}</span>}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Implement KeyInput**

Create `src-ui/src/components/common/KeyInput.tsx`:

```tsx
import { useState } from "react";

interface KeyInputProps {
  label: string;
  value: string;
  onChange: (key: string) => void;
  disabled?: boolean;
}

export function KeyInput({ label, value, onChange, disabled = false }: KeyInputProps) {
  const [listening, setListening] = useState(false);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    e.preventDefault();
    if (listening) {
      onChange(e.key.length === 1 ? e.key.toUpperCase() : e.key);
      setListening(false);
    }
  };

  return (
    <div className="space-y-1">
      <label className="text-xs text-subtle">{label}</label>
      <button
        type="button"
        disabled={disabled}
        onClick={() => setListening(true)}
        onKeyDown={handleKeyDown}
        onBlur={() => setListening(false)}
        className={`
          flex h-8 w-full items-center rounded-md border px-3 font-mono text-sm
          transition-colors
          ${listening
            ? "border-border-accent bg-elevated text-gold animate-pulse"
            : "border-border bg-input text-content"
          }
          ${disabled ? "cursor-not-allowed opacity-50" : "cursor-pointer"}
        `}
      >
        {listening ? "Press a key..." : value || "—"}
      </button>
    </div>
  );
}
```

- [ ] **Step 4: Run tests**

```bash
cd src-ui && npm test -- NumberInput
```

Expected: All 2 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(ui): add NumberInput and KeyInput components"
```

---

### Task 9: TagList Component

**Files:**
- Create: `src-ui/src/components/common/TagList.tsx`
- Create: `src-ui/src/components/common/TagList.test.tsx`

- [ ] **Step 1: Write the TagList test**

Create `src-ui/src/components/common/TagList.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TagList } from "./TagList";

describe("TagList", () => {
  it("renders tags", () => {
    render(<TagList label="Items" items={["orchid", "bloodthorn"]} onChange={() => {}} />);
    expect(screen.getByText("orchid")).toBeInTheDocument();
    expect(screen.getByText("bloodthorn")).toBeInTheDocument();
  });

  it("removes a tag when × is clicked", async () => {
    const onChange = vi.fn();
    render(<TagList label="Items" items={["orchid", "bloodthorn"]} onChange={onChange} />);
    const removeButtons = screen.getAllByRole("button", { name: /remove/i });
    await userEvent.click(removeButtons[0]);
    expect(onChange).toHaveBeenCalledWith(["bloodthorn"]);
  });

  it("adds a tag via input", async () => {
    const onChange = vi.fn();
    render(<TagList label="Items" items={["orchid"]} onChange={onChange} />);
    const addBtn = screen.getByRole("button", { name: /add/i });
    await userEvent.click(addBtn);
    const input = screen.getByPlaceholderText("Add item...");
    await userEvent.type(input, "nullifier{enter}");
    expect(onChange).toHaveBeenCalledWith(["orchid", "nullifier"]);
  });
});
```

- [ ] **Step 2: Implement TagList**

Create `src-ui/src/components/common/TagList.tsx`:

```tsx
import { useState } from "react";
import { Plus, X } from "lucide-react";

interface TagListProps {
  label: string;
  items: string[];
  onChange: (items: string[]) => void;
  disabled?: boolean;
}

export function TagList({ label, items, onChange, disabled = false }: TagListProps) {
  const [adding, setAdding] = useState(false);
  const [draft, setDraft] = useState("");

  const remove = (index: number) => {
    onChange(items.filter((_, i) => i !== index));
  };

  const add = () => {
    const trimmed = draft.trim();
    if (trimmed && !items.includes(trimmed)) {
      onChange([...items, trimmed]);
    }
    setDraft("");
    setAdding(false);
  };

  return (
    <div className="space-y-2">
      <label className="text-xs text-subtle">{label}</label>
      <div className="flex flex-wrap gap-2">
        {items.map((item, i) => (
          <span
            key={item}
            className="inline-flex items-center gap-1 rounded-full border border-border bg-elevated px-2.5 py-0.5 text-xs text-content"
          >
            {item}
            {!disabled && (
              <button
                type="button"
                onClick={() => remove(i)}
                aria-label={`remove ${item}`}
                className="ml-0.5 rounded-full p-0.5 text-muted hover:text-danger"
              >
                <X className="h-3 w-3" />
              </button>
            )}
          </span>
        ))}
        {!disabled &&
          (adding ? (
            <input
              autoFocus
              placeholder="Add item..."
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && add()}
              onBlur={() => { add(); setAdding(false); }}
              className="h-6 w-28 rounded-full border border-dashed border-border bg-input px-2 text-xs text-content focus:outline-none"
            />
          ) : (
            <button
              type="button"
              onClick={() => setAdding(true)}
              aria-label="add"
              className="inline-flex items-center gap-1 rounded-full border border-dashed border-border px-2.5 py-0.5 text-xs text-muted hover:text-content"
            >
              <Plus className="h-3 w-3" /> Add
            </button>
          ))}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Run tests**

```bash
cd src-ui && npm test -- TagList
```

Expected: All 3 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(ui): add TagList component"
```

---

### Task 10: HPBar and ManaBar Components

**Files:**
- Create: `src-ui/src/components/common/HPBar.tsx`
- Create: `src-ui/src/components/common/ManaBar.tsx`
- Create: `src-ui/src/components/common/HPBar.test.tsx`

- [ ] **Step 1: Write the HPBar test**

Create `src-ui/src/components/common/HPBar.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { HPBar } from "./HPBar";
import { ManaBar } from "./ManaBar";

describe("HPBar", () => {
  it("renders percentage text", () => {
    render(<HPBar percent={75} />);
    expect(screen.getByText("75%")).toBeInTheDocument();
  });

  it("applies green color at high HP", () => {
    const { container } = render(<HPBar percent={80} />);
    const fill = container.querySelector("[data-fill]");
    expect(fill).toHaveStyle({ width: "80%" });
  });

  it("applies danger color at low HP", () => {
    const { container } = render(<HPBar percent={20} />);
    const fill = container.querySelector("[data-fill]");
    expect(fill?.className).toContain("bg-danger");
  });
});

describe("ManaBar", () => {
  it("renders percentage text", () => {
    render(<ManaBar percent={60} />);
    expect(screen.getByText("60%")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Implement HPBar**

Create `src-ui/src/components/common/HPBar.tsx`:

```tsx
interface HPBarProps {
  percent: number;
  size?: "sm" | "md";
}

function hpColor(pct: number): string {
  if (pct > 60) return "bg-success";
  if (pct > 30) return "bg-warning";
  return "bg-danger";
}

export function HPBar({ percent, size = "sm" }: HPBarProps) {
  const h = size === "sm" ? "h-2" : "h-4";
  return (
    <div className={`relative w-full ${h} overflow-hidden rounded-full bg-input`}>
      <div
        data-fill
        className={`absolute left-0 top-0 ${h} rounded-full transition-all duration-300 ${hpColor(percent)}`}
        style={{ width: `${Math.max(0, Math.min(100, percent))}%` }}
      />
      <span className="absolute inset-0 flex items-center justify-center text-[10px] font-mono font-medium text-content drop-shadow">
        {percent}%
      </span>
    </div>
  );
}
```

- [ ] **Step 3: Implement ManaBar**

Create `src-ui/src/components/common/ManaBar.tsx`:

```tsx
interface ManaBarProps {
  percent: number;
  size?: "sm" | "md";
}

export function ManaBar({ percent, size = "sm" }: ManaBarProps) {
  const h = size === "sm" ? "h-2" : "h-4";
  return (
    <div className={`relative w-full ${h} overflow-hidden rounded-full bg-input`}>
      <div
        data-fill
        className={`absolute left-0 top-0 ${h} rounded-full bg-info transition-all duration-300`}
        style={{ width: `${Math.max(0, Math.min(100, percent))}%` }}
      />
      <span className="absolute inset-0 flex items-center justify-center text-[10px] font-mono font-medium text-content drop-shadow">
        {percent}%
      </span>
    </div>
  );
}
```

- [ ] **Step 4: Run tests**

```bash
cd src-ui && npm test -- HPBar
```

Expected: All 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(ui): add HPBar and ManaBar components"
```

---

### Task 11: DangerBadge and Dropdown Components

**Files:**
- Create: `src-ui/src/components/common/DangerBadge.tsx`
- Create: `src-ui/src/components/common/Dropdown.tsx`
- Create: `src-ui/src/components/common/DangerBadge.test.tsx`

- [ ] **Step 1: Write the DangerBadge test**

Create `src-ui/src/components/common/DangerBadge.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { DangerBadge } from "./DangerBadge";

describe("DangerBadge", () => {
  it("renders danger text", () => {
    render(<DangerBadge />);
    expect(screen.getByText("⚠ DANGER")).toBeInTheDocument();
  });

  it("renders custom text", () => {
    render(<DangerBadge text="CRITICAL" />);
    expect(screen.getByText("CRITICAL")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Implement DangerBadge**

Create `src-ui/src/components/common/DangerBadge.tsx`:

```tsx
interface DangerBadgeProps {
  text?: string;
}

export function DangerBadge({ text = "⚠ DANGER" }: DangerBadgeProps) {
  return (
    <span className="inline-flex items-center rounded bg-danger px-2 py-0.5 text-xs font-semibold text-white animate-pulse">
      {text}
    </span>
  );
}
```

- [ ] **Step 3: Implement Dropdown**

Create `src-ui/src/components/common/Dropdown.tsx`:

```tsx
import { ChevronDown } from "lucide-react";

interface DropdownProps {
  label: string;
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
  disabled?: boolean;
}

export function Dropdown({
  label,
  value,
  options,
  onChange,
  disabled = false,
}: DropdownProps) {
  return (
    <div className="space-y-1">
      <label className="text-xs text-subtle">{label}</label>
      <div className="relative">
        <select
          value={value}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled}
          className="h-8 w-full appearance-none rounded-md border border-border bg-input px-3 pr-8
                     font-mono text-sm text-content focus:border-border-accent focus:outline-none
                     disabled:cursor-not-allowed disabled:opacity-50"
        >
          {options.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
        <ChevronDown className="pointer-events-none absolute right-2 top-2 h-4 w-4 text-muted" />
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Run tests**

```bash
cd src-ui && npm test -- DangerBadge
```

Expected: All 2 tests PASS.

- [ ] **Step 5: Create barrel export for all common components**

Create `src-ui/src/components/common/index.ts`:

```ts
export { Toggle } from "./Toggle";
export { Card } from "./Card";
export { Button } from "./Button";
export { Slider } from "./Slider";
export { NumberInput } from "./NumberInput";
export { KeyInput } from "./KeyInput";
export { TagList } from "./TagList";
export { HPBar } from "./HPBar";
export { ManaBar } from "./ManaBar";
export { DangerBadge } from "./DangerBadge";
export { Dropdown } from "./Dropdown";
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(ui): add DangerBadge, Dropdown, and barrel exports"
```

---

## Phase 3: Layout Components

### Task 12: Sidebar Navigation

**Files:**
- Create: `src-ui/src/components/layout/Sidebar.tsx`
- Create: `src-ui/src/components/layout/Sidebar.test.tsx`

- [ ] **Step 1: Write the Sidebar test**

Create `src-ui/src/components/layout/Sidebar.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { Sidebar } from "./Sidebar";

function renderSidebar() {
  return render(
    <MemoryRouter initialEntries={["/"]}>
      <Sidebar />
    </MemoryRouter>,
  );
}

describe("Sidebar", () => {
  it("renders all navigation items", () => {
    renderSidebar();
    expect(screen.getByText("Dashboard")).toBeInTheDocument();
    expect(screen.getByText("Heroes")).toBeInTheDocument();
    expect(screen.getByText("Danger")).toBeInTheDocument();
    expect(screen.getByText("Soul Ring")).toBeInTheDocument();
    expect(screen.getByText("Armlet")).toBeInTheDocument();
    expect(screen.getByText("Activity")).toBeInTheDocument();
    expect(screen.getByText("Diagnostics")).toBeInTheDocument();
    expect(screen.getByText("Settings")).toBeInTheDocument();
  });

  it("renders version in footer", () => {
    renderSidebar();
    expect(screen.getByText(/v\d/)).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Implement Sidebar**

Create `src-ui/src/components/layout/Sidebar.tsx`:

```tsx
import { NavLink } from "react-router-dom";
import {
  LayoutDashboard,
  Swords,
  Shield,
  CircleDot,
  Axe,
  ScrollText,
  Activity,
  Settings,
} from "lucide-react";

const navItems = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard },
  { to: "/heroes", label: "Heroes", icon: Swords },
  { to: "/danger", label: "Danger", icon: Shield },
  { to: "/soul-ring", label: "Soul Ring", icon: CircleDot },
  { to: "/armlet", label: "Armlet", icon: Axe },
  { to: "/activity", label: "Activity", icon: ScrollText },
  { to: "/diagnostics", label: "Diagnostics", icon: Activity },
  { to: "/settings", label: "Settings", icon: Settings },
];

export function Sidebar() {
  return (
    <aside className="flex h-full w-[200px] shrink-0 flex-col border-r border-border bg-base">
      <div className="p-4">
        <h1 className="text-lg font-semibold text-gold">D2 Scripts</h1>
      </div>
      <nav className="flex-1 space-y-0.5 px-2">
        {navItems.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            end={to === "/"}
            className={({ isActive }) =>
              `flex items-center gap-3 rounded-md px-3 py-2.5 text-sm transition-colors ${
                isActive
                  ? "border-l-[3px] border-gold bg-elevated text-gold"
                  : "border-l-[3px] border-transparent text-subtle hover:bg-elevated hover:text-content"
              }`
            }
          >
            <Icon className="h-5 w-5 shrink-0" />
            <span>{label}</span>
          </NavLink>
        ))}
      </nav>
      <div className="border-t border-border p-4">
        <span className="text-xs text-muted">v{appVersion}</span>
      </div>
    </aside>
  );
}
```

- [ ] **Step 3: Run tests**

```bash
cd src-ui && npm test -- Sidebar
```

Expected: All 2 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(ui): add Sidebar navigation component"
```

---

### Task 13: StatusHeader Component

**Files:**
- Create: `src-ui/src/components/layout/StatusHeader.tsx`
- Create: `src-ui/src/components/layout/StatusHeader.test.tsx`

- [ ] **Step 1: Write the StatusHeader test**

Create `src-ui/src/components/layout/StatusHeader.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { StatusHeader } from "./StatusHeader";

describe("StatusHeader", () => {
  it("renders idle state when no game data", () => {
    render(<StatusHeader />);
    expect(screen.getByText("Waiting for game...")).toBeInTheDocument();
  });

  it("renders in-game state with hero info", () => {
    render(
      <StatusHeader
        heroName="Shadow Fiend"
        heroLevel={15}
        hpPercent={72}
        manaPercent={55}
        inDanger={false}
        connected={true}
      />,
    );
    expect(screen.getByText("Shadow Fiend")).toBeInTheDocument();
    expect(screen.getByText("Lv. 15")).toBeInTheDocument();
    expect(screen.getByText("72%")).toBeInTheDocument();
    expect(screen.getByText("55%")).toBeInTheDocument();
  });

  it("shows danger badge when in danger", () => {
    render(
      <StatusHeader
        heroName="Huskar"
        heroLevel={10}
        hpPercent={20}
        manaPercent={40}
        inDanger={true}
        connected={true}
      />,
    );
    expect(screen.getByText("⚠ DANGER")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Implement StatusHeader**

Create `src-ui/src/components/layout/StatusHeader.tsx`:

```tsx
import { HPBar } from "../common/HPBar";
import { ManaBar } from "../common/ManaBar";
import { DangerBadge } from "../common/DangerBadge";
import { Wifi, WifiOff } from "lucide-react";

interface StatusHeaderProps {
  heroName?: string;
  heroLevel?: number;
  hpPercent?: number;
  manaPercent?: number;
  inDanger?: boolean;
  connected?: boolean;
  runeTimer?: number | null;
  stunned: boolean;
  silenced: boolean;
  alive: boolean;
  respawnTimer: number | null;
}

export function StatusHeader({
  heroName,
  heroLevel,
  hpPercent,
  manaPercent,
  inDanger = false,
  connected = false,
  runeTimer,
  stunned,
  silenced,
  alive,
  respawnTimer,
}: StatusHeaderProps) {
  const inGame = !!heroName;

  return (
    <header className="flex h-12 shrink-0 items-center gap-4 border-b border-border bg-surface px-4">
      {inGame ? (
        <>
          <div className="flex items-center gap-2">
            <span className="font-semibold text-content">{heroName}</span>
            <span className="rounded bg-elevated px-1.5 py-0.5 font-mono text-xs text-subtle">
              Lv. {heroLevel}
            </span>
          </div>
          <div className="flex items-center gap-3 flex-1">
            <div className="w-32">
              <HPBar percent={hpPercent ?? 0} />
            </div>
            <div className="w-28">
              <ManaBar percent={manaPercent ?? 0} />
            </div>
            {inDanger && <DangerBadge />}
            {!alive && (
              <div className="flex items-center gap-1 text-danger text-xs font-mono">
                <span>💀</span>
                {respawnTimer !== null && <span>{respawnTimer}s</span>}
              </div>
            )}
            {stunned && <span className="text-warning text-xs">⚡ Stunned</span>}
            {silenced && <span className="text-danger text-xs">🔇 Silenced</span>}
            {runeTimer != null && runeTimer <= 15 && (
              <span className="font-mono text-xs text-warning animate-pulse">
                🔮 {runeTimer}s
              </span>
            )}
          </div>
          <div className="flex items-center gap-1">
            {connected ? (
              <Wifi className="h-4 w-4 text-success" />
            ) : (
              <WifiOff className="h-4 w-4 text-danger" />
            )}
          </div>
        </>
      ) : (
        <>
          <span className="text-sm font-semibold text-content">D2 Scripts</span>
          <div className="flex items-center gap-2">
            <span className="h-2 w-2 rounded-full bg-subtle animate-pulse" />
            <span className="text-xs text-subtle">Waiting for game...</span>
          </div>
          <div className="flex-1" />
          <span className="text-xs text-muted">v0.1.0-dev</span>
        </>
      )}
    </header>
  );
}
```

- [ ] **Step 3: Run tests**

```bash
cd src-ui && npm test -- StatusHeader
```

Expected: All 3 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(ui): add StatusHeader component"
```

---

### Task 14: ActivityTicker Component

**Files:**
- Create: `src-ui/src/components/layout/ActivityTicker.tsx`
- Create: `src-ui/src/components/layout/index.ts`

- [ ] **Step 1: Implement ActivityTicker**

Create `src-ui/src/components/layout/ActivityTicker.tsx`:

```tsx
import { useState } from "react";
import { Link } from "react-router-dom";
import { ChevronUp, ChevronDown } from "lucide-react";

export interface TickerEntry {
  id: string;
  timestamp: string;
  category: "action" | "danger" | "warning" | "system";
  message: string;
}

interface ActivityTickerProps {
  entries: TickerEntry[];
}

const categoryColors: Record<string, string> = {
  action: "text-terminal",
  danger: "text-danger",
  warning: "text-warning",
  system: "text-info",
};

export function ActivityTicker({ entries }: ActivityTickerProps) {
  const [expanded, setExpanded] = useState(false);
  const visible = expanded ? entries.slice(-3) : entries.slice(-1);

  return (
    <div className="shrink-0 border-t border-border bg-terminal-bg px-4 py-1">
      <div className="flex items-center justify-between">
        <div className="flex-1 overflow-hidden">
          {visible.map((entry) => (
            <div key={entry.id} className="flex items-center gap-2 font-mono text-xs">
              <span className="text-muted">{entry.timestamp}</span>
              <span className={categoryColors[entry.category]}>{entry.message}</span>
            </div>
          ))}
        </div>
        <div className="flex items-center gap-2 ml-2">
          <Link to="/activity" className="text-xs text-gold hover:underline">
            View All
          </Link>
          <button
            type="button"
            onClick={() => setExpanded(!expanded)}
            className="text-muted hover:text-content"
          >
            {expanded ? <ChevronDown className="h-3 w-3" /> : <ChevronUp className="h-3 w-3" />}
          </button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Create layout barrel export**

Create `src-ui/src/components/layout/index.ts`:

```ts
export { Sidebar } from "./Sidebar";
export { StatusHeader } from "./StatusHeader";
export { ActivityTicker } from "./ActivityTicker";
export type { TickerEntry } from "./ActivityTicker";
```

- [ ] **Step 3: Verify build**

```bash
cd src-ui && npm run build
```

Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(ui): add ActivityTicker and layout barrel exports"
```

---

## Phase 4: Data Layer

### Task 15: Type Definitions

**Files:**
- Create: `src-ui/src/types/config.ts`
- Create: `src-ui/src/types/game.ts`
- Create: `src-ui/src/types/activity.ts`
- Create: `src-ui/src/types/index.ts`

These types mirror the Rust `Settings` struct from `src/config/settings.rs` and `AppState` from `src/state/app_state.rs`.

- [ ] **Step 1: Create config types**

Create `src-ui/src/types/config.ts`:

```ts
export interface ServerConfig {
  port: number;
}

export interface UpdateConfig {
  check_on_startup: boolean;
  include_prereleases: boolean;
}

export interface KeybindingsConfig {
  slot0: string;
  slot1: string;
  slot2: string;
  slot3: string;
  slot4: string;
  slot5: string;
  neutral0: string;
  combo_trigger: string;
}

export interface LoggingConfig {
  level: "debug" | "info" | "warn" | "error";
}

export interface CommonConfig {
  survivability_hp_threshold: number;
}

export interface ArmletConfig {
  enabled: boolean;
  cast_modifier: string;
  toggle_threshold: number;
  predictive_offset: number;
  toggle_cooldown_ms: number;
}

export interface HeroArmletOverride {
  enabled?: boolean;
  toggle_threshold?: number;
  predictive_offset?: number;
  toggle_cooldown_ms?: number;
}

export interface HuskarConfig {
  armlet_toggle_threshold: number;
  armlet_predictive_offset: number;
  armlet_toggle_cooldown_ms: number;
  berserker_blood_key: string;
  berserker_blood_delay_ms: number;
  standalone_key: string;
  armlet: HeroArmletOverride;
}

export interface LegionCommanderConfig {
  standalone_key: string;
  armlet: HeroArmletOverride;
}

export interface ShadowFiendConfig {
  raze_intercept_enabled: boolean;
  raze_delay_ms: number;
  auto_bkb_on_ultimate: boolean;
  auto_d_on_ultimate: boolean;
  standalone_key: string;
  armlet: HeroArmletOverride;
}

export interface TinyConfig {
  standalone_key: string;
  armlet: HeroArmletOverride;
}

export interface AutoAbilityConfig {
  index: number;
  key: string;
  hp_threshold?: number;
}

export interface BroodmotherConfig {
  spider_micro_enabled: boolean;
  spider_control_group_key: string;
  reselect_hero_key: string;
  attack_key: string;
  standalone_key: string;
  auto_items_enabled: boolean;
  auto_items_modifier: string;
  auto_items: string[];
  auto_abilities: AutoAbilityConfig[];
  auto_abilities_first: boolean;
  armlet: HeroArmletOverride;
}

export interface LargoConfig {
  amphibian_rhapsody_enabled: boolean;
  auto_toggle_on_danger: boolean;
  mana_threshold_percent: number;
  heal_hp_threshold: number;
  beat_interval_ms: number;
  beat_correction_ms: number;
  beat_correction_every_n_beats: number;
  q_ability_key: string;
  w_ability_key: string;
  e_ability_key: string;
  r_ability_key: string;
  standalone_key: string;
  armlet: HeroArmletOverride;
}

export interface MeepoFarmAssistConfig {
  enabled: boolean;
  toggle_key: string;
  pulse_interval_ms: number;
  minimum_mana_percent: number;
  minimum_health_percent: number;
  right_click_after_poof: boolean;
  suspend_on_danger: boolean;
  suspend_after_manual_combo_ms: number;
  poof_press_count: number;
  poof_press_interval_ms: number;
}

export interface MeepoConfig {
  standalone_key: string;
  earthbind_key: string;
  poof_key: string;
  dig_key: string;
  megameepo_key: string;
  post_blink_delay_ms: number;
  combo_items: string[];
  combo_item_spam_count: number;
  combo_item_delay_ms: number;
  earthbind_press_count: number;
  earthbind_press_interval_ms: number;
  poof_press_count: number;
  poof_press_interval_ms: number;
  auto_dig_on_danger: boolean;
  dig_hp_threshold_percent: number;
  auto_megameepo_on_danger: boolean;
  megameepo_hp_threshold_percent: number;
  defensive_trigger_cooldown_ms: number;
  farm_assist: MeepoFarmAssistConfig;
  armlet: HeroArmletOverride;
}

export interface OutworldDestroyerConfig {
  standalone_key: string;
  objurgation_key: string;
  arcane_orb_key: string;
  astral_imprisonment_key: string;
  auto_objurgation_on_danger: boolean;
  objurgation_hp_threshold_percent: number;
  objurgation_min_mana_percent: number;
  objurgation_trigger_cooldown_ms: number;
  ultimate_intercept_enabled: boolean;
  auto_bkb_on_ultimate: boolean;
  auto_objurgation_on_ultimate: boolean;
  post_bkb_delay_ms: number;
  post_blink_delay_ms: number;
  astral_self_cast_enabled: boolean;
  astral_self_cast_key: string;
  combo_items: string[];
  combo_item_spam_count: number;
  combo_item_delay_ms: number;
  post_ultimate_arcane_orb_presses: number;
  arcane_orb_press_interval_ms: number;
  armlet: HeroArmletOverride;
}

export interface HeroesConfig {
  huskar: HuskarConfig;
  legion_commander: LegionCommanderConfig;
  shadow_fiend: ShadowFiendConfig;
  tiny: TinyConfig;
  outworld_destroyer: OutworldDestroyerConfig;
  largo: LargoConfig;
  broodmother: BroodmotherConfig;
  meepo: MeepoConfig;
}

export interface DangerDetectionConfig {
  enabled: boolean;
  hp_threshold_percent: number;
  rapid_loss_hp: number;
  time_window_ms: number;
  clear_delay_seconds: number;
  healing_threshold_in_danger: number;
  max_healing_items_per_danger: number;
  auto_bkb: boolean;
  auto_satanic: boolean;
  satanic_hp_threshold: number;
  auto_blade_mail: boolean;
  auto_glimmer_cape: boolean;
  auto_ghost_scepter: boolean;
  auto_shivas_guard: boolean;
  auto_manta_on_silence: boolean;
  auto_lotus_on_silence: boolean;
}

export interface NeutralItemConfig {
  enabled: boolean;
  self_cast_key: string;
  log_discoveries: boolean;
  use_in_danger: boolean;
  hp_threshold: number;
  allowed_items: string[];
}

export interface SoulRingConfig {
  enabled: boolean;
  min_mana_percent: number;
  min_health_percent: number;
  delay_before_ability_ms: number;
  trigger_cooldown_ms: number;
  ability_keys: string[];
  intercept_item_keys: boolean;
}

export interface RuneAlertConfig {
  enabled: boolean;
  alert_lead_seconds: number;
  interval_seconds: number;
  audio_enabled: boolean;
}

export interface MinimapCaptureConfig {
  enabled: boolean;
  minimap_x: number;
  minimap_y: number;
  minimap_width: number;
  minimap_height: number;
  capture_interval_ms: number;
  sample_every_n: number;
  artifact_output_dir: string;
}

export interface Settings {
  server: ServerConfig;
  keybindings: KeybindingsConfig;
  logging: LoggingConfig;
  common: CommonConfig;
  armlet: ArmletConfig;
  heroes: HeroesConfig;
  danger_detection: DangerDetectionConfig;
  neutral_items: NeutralItemConfig;
  soul_ring: SoulRingConfig;
  updates: UpdateConfig;
  rune_alerts: RuneAlertConfig;
  minimap_capture: MinimapCaptureConfig;
}
```

- [ ] **Step 2: Create game types**

Create `src-ui/src/types/game.ts`:

```ts
export type HeroType =
  | "broodmother"
  | "huskar"
  | "largo"
  | "legion_commander"
  | "meepo"
  | "outworld_destroyer"
  | "shadow_fiend"
  | "tiny";

export interface HeroInfo {
  id: HeroType;
  displayName: string;
  internalName: string;
  icon: string;
  role: string;
}

export const HEROES: HeroInfo[] = [
  { id: "broodmother", displayName: "Broodmother", internalName: "npc_dota_hero_broodmother", icon: "🕷️", role: "Pusher / Carry" },
  { id: "huskar", displayName: "Huskar", internalName: "npc_dota_hero_huskar", icon: "🔥", role: "Carry / Durable" },
  { id: "largo", displayName: "Largo", internalName: "npc_dota_hero_largo", icon: "🎵", role: "Support / Healer" },
  { id: "legion_commander", displayName: "Legion Commander", internalName: "npc_dota_hero_legion_commander", icon: "⚔️", role: "Initiator / Durable" },
  { id: "meepo", displayName: "Meepo", internalName: "npc_dota_hero_meepo", icon: "🐾", role: "Carry / Escape" },
  { id: "outworld_destroyer", displayName: "Outworld Destroyer", internalName: "npc_dota_hero_obsidian_destroyer", icon: "🌀", role: "Carry / Nuker" },
  { id: "shadow_fiend", displayName: "Shadow Fiend", internalName: "npc_dota_hero_nevermore", icon: "👻", role: "Carry / Nuker" },
  { id: "tiny", displayName: "Tiny", internalName: "npc_dota_hero_tiny", icon: "🪨", role: "Initiator / Nuker" },
];

export type UpdateCheckState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "available"; version: string; releaseNotes?: string }
  | { kind: "downloading" }
  | { kind: "error"; message: string }
  | { kind: "upToDate" };

export interface GameState {
  heroName: string | null;
  heroLevel: number;
  hpPercent: number;
  manaPercent: number;
  inDanger: boolean;
  connected: boolean;
  alive: boolean;
  stunned: boolean;
  silenced: boolean;
  respawnTimer: number | null;
  runeTimer: number | null;
  gameTime: number;
}

export interface QueueMetrics {
  eventsProcessed: number;
  eventsDropped: number;
  currentQueueDepth: number;
  maxQueueDepth: number;
}

export interface DiagnosticsState {
  gsiConnected: boolean;
  keyboardHookActive: boolean;
  queueMetrics: QueueMetrics;
  syntheticInput: {
    queueDepth: number;
    totalQueued: number;
    peakDepth: number;
    completions: number;
    drops: number;
  };
  soulRingState: "ready" | "triggered" | "cooldown";
  blockedKeys: string[];
}
```

- [ ] **Step 3: Create activity types**

Create `src-ui/src/types/activity.ts`:

```ts
export type ActivityCategory = "action" | "danger" | "warning" | "error" | "system";

export interface ActivityEntry {
  id: string;
  timestamp: string;
  category: ActivityCategory;
  message: string;
  details?: string;
}
```

- [ ] **Step 4: Create types barrel export**

Create `src-ui/src/types/index.ts`:

```ts
export type * from "./config";
export type * from "./game";
export type * from "./activity";
export { HEROES } from "./game";
```

- [ ] **Step 5: Verify build**

```bash
cd src-ui && npm run build
```

Expected: Build succeeds.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(ui): add TypeScript type definitions"
```

---

### Task 16: Zustand Stores with Mock Data

**Files:**
- Create: `src-ui/src/stores/configStore.ts`
- Create: `src-ui/src/stores/gameStore.ts`
- Create: `src-ui/src/stores/uiStore.ts`
- Create: `src-ui/src/stores/activityStore.ts`
- Create: `src-ui/src/stores/index.ts`
- Create: `src-ui/src/stores/mockData.ts`

- [ ] **Step 1: Create mock data**

Create `src-ui/src/stores/mockData.ts`:

This file contains realistic default config values matching `config/config.toml`.

```ts
import type { Settings } from "../types/config";
import type { ActivityEntry } from "../types/activity";

export const mockConfig: Settings = {
  server: { port: 3000 },
  keybindings: {
    slot0: "z", slot1: "x", slot2: "c", slot3: "v", slot4: "b", slot5: "n",
    neutral0: "0", combo_trigger: "Home",
  },
  logging: { level: "info" },
  common: { survivability_hp_threshold: 30 },
  armlet: {
    enabled: true, cast_modifier: "Alt", toggle_threshold: 320,
    predictive_offset: 30, toggle_cooldown_ms: 250,
  },
  heroes: {
    huskar: {
      armlet_toggle_threshold: 120, armlet_predictive_offset: 150,
      armlet_toggle_cooldown_ms: 300, berserker_blood_key: "e",
      berserker_blood_delay_ms: 300, standalone_key: "Home",
      armlet: {},
    },
    legion_commander: { standalone_key: "Home", armlet: {} },
    shadow_fiend: {
      raze_intercept_enabled: true, raze_delay_ms: 10,
      auto_bkb_on_ultimate: true, auto_d_on_ultimate: true,
      standalone_key: "Home", armlet: {},
    },
    tiny: { standalone_key: "Home", armlet: {} },
    outworld_destroyer: {
      standalone_key: "Home", objurgation_key: "w", arcane_orb_key: "q",
      astral_imprisonment_key: "e", auto_objurgation_on_danger: true,
      objurgation_hp_threshold_percent: 55, objurgation_min_mana_percent: 25,
      objurgation_trigger_cooldown_ms: 1500, ultimate_intercept_enabled: true,
      auto_bkb_on_ultimate: true, auto_objurgation_on_ultimate: true,
      post_bkb_delay_ms: 50, post_blink_delay_ms: 80,
      astral_self_cast_enabled: true, astral_self_cast_key: "F5",
      combo_items: ["sheepstick", "bloodthorn"], combo_item_spam_count: 3,
      combo_item_delay_ms: 30, post_ultimate_arcane_orb_presses: 3,
      arcane_orb_press_interval_ms: 50, armlet: {},
    },
    largo: {
      amphibian_rhapsody_enabled: true, auto_toggle_on_danger: true,
      mana_threshold_percent: 20, heal_hp_threshold: 50,
      beat_interval_ms: 995, beat_correction_ms: 30,
      beat_correction_every_n_beats: 5, q_ability_key: "q",
      w_ability_key: "w", e_ability_key: "e", r_ability_key: "r",
      standalone_key: "Home", armlet: {},
    },
    broodmother: {
      spider_micro_enabled: true, spider_control_group_key: "F3",
      reselect_hero_key: "1", attack_key: "a", standalone_key: "Space",
      auto_items_enabled: true, auto_items_modifier: "Space",
      auto_items: ["orchid", "bloodthorn", "diffusal_blade", "disperser", "nullifier", "abyssal_blade"],
      auto_abilities: [
        { index: 0, key: "q", hp_threshold: 80 },
        { index: 3, key: "r" },
      ],
      auto_abilities_first: false, armlet: {},
    },
    meepo: {
      standalone_key: "Home", earthbind_key: "q", poof_key: "w",
      dig_key: "e", megameepo_key: "r", post_blink_delay_ms: 80,
      combo_items: ["sheepstick", "disperser"], combo_item_spam_count: 3,
      combo_item_delay_ms: 30, earthbind_press_count: 2,
      earthbind_press_interval_ms: 50, poof_press_count: 3,
      poof_press_interval_ms: 50, auto_dig_on_danger: true,
      dig_hp_threshold_percent: 32, auto_megameepo_on_danger: true,
      megameepo_hp_threshold_percent: 45, defensive_trigger_cooldown_ms: 1500,
      farm_assist: {
        enabled: true, toggle_key: "End", pulse_interval_ms: 700,
        minimum_mana_percent: 35, minimum_health_percent: 45,
        right_click_after_poof: true, suspend_on_danger: true,
        suspend_after_manual_combo_ms: 2500, poof_press_count: 3,
        poof_press_interval_ms: 50,
      },
      armlet: {},
    },
  },
  danger_detection: {
    enabled: true, hp_threshold_percent: 70, rapid_loss_hp: 100,
    time_window_ms: 500, clear_delay_seconds: 3,
    healing_threshold_in_danger: 50, max_healing_items_per_danger: 3,
    auto_bkb: true, auto_satanic: true, satanic_hp_threshold: 40,
    auto_blade_mail: true, auto_glimmer_cape: true,
    auto_ghost_scepter: true, auto_shivas_guard: true,
    auto_manta_on_silence: true, auto_lotus_on_silence: true,
  },
  neutral_items: {
    enabled: false, self_cast_key: "0", log_discoveries: false,
    use_in_danger: true, hp_threshold: 50,
    allowed_items: ["essence_ring", "minotaur_horn", "metamorphic_mandible"],
  },
  soul_ring: {
    enabled: true, min_mana_percent: 100, min_health_percent: 20,
    delay_before_ability_ms: 30, trigger_cooldown_ms: 10,
    ability_keys: ["q", "w", "e", "r", "d", "f"],
    intercept_item_keys: true,
  },
  updates: { check_on_startup: true, include_prereleases: false },
  rune_alerts: {
    enabled: true, alert_lead_seconds: 10,
    interval_seconds: 120, audio_enabled: true,
  },
  minimap_capture: {
    enabled: false, minimap_x: 0, minimap_y: 0,
    minimap_width: 256, minimap_height: 256,
    capture_interval_ms: 1000, sample_every_n: 5,
    artifact_output_dir: "artifacts/minimap",
  },
};

export const mockActivityLog: ActivityEntry[] = [
  { id: "1", timestamp: "14:32:01.234", category: "system", message: "GSI server started on port 3000" },
  { id: "2", timestamp: "14:32:05.112", category: "system", message: "Hero detected: Shadow Fiend" },
  { id: "3", timestamp: "14:33:12.456", category: "action", message: "Soul Ring → Raze (Q)" },
  { id: "4", timestamp: "14:33:15.789", category: "danger", message: "⚠ Danger detected — HP 28%" },
  { id: "5", timestamp: "14:33:15.820", category: "action", message: "Auto-BKB activated" },
  { id: "6", timestamp: "14:33:16.100", category: "action", message: "Satanic activated (HP 22%)" },
  { id: "7", timestamp: "14:34:00.000", category: "system", message: "🔮 Rune spawning in 10s" },
  { id: "8", timestamp: "14:35:22.333", category: "action", message: "Armlet toggled ON" },
];
```

- [ ] **Step 2: Create configStore**

Create `src-ui/src/stores/configStore.ts`:

```ts
import { create } from "zustand";
import type { Settings } from "../types/config";
import { mockConfig } from "./mockData";

interface ConfigStore {
  config: Settings;
  updateConfig: <K extends keyof Settings>(
    section: K,
    updates: Partial<Settings[K]>,
  ) => void;
  updateHeroConfig: <K extends keyof Settings["heroes"]>(
    hero: K,
    updates: Partial<Settings["heroes"][K]>,
  ) => void;
}

export const useConfigStore = create<ConfigStore>((set) => ({
  config: mockConfig,

  updateConfig: (section, updates) =>
    set((state) => ({
      config: {
        ...state.config,
        [section]: { ...state.config[section], ...updates },
      },
    })),

  updateHeroConfig: (hero, updates) =>
    set((state) => ({
      config: {
        ...state.config,
        heroes: {
          ...state.config.heroes,
          [hero]: { ...state.config.heroes[hero], ...updates },
        },
      },
    })),
}));
```

- [ ] **Step 3: Create gameStore**

Create `src-ui/src/stores/gameStore.ts`:

```ts
import { create } from "zustand";
import type { GameState, DiagnosticsState, UpdateCheckState } from "../types/game";

interface GameStore {
  game: GameState;
  diagnostics: DiagnosticsState;
  updateState: UpdateCheckState;
  setGame: (game: Partial<GameState>) => void;
}

export const useGameStore = create<GameStore>((set) => ({
  game: {
    heroName: null,
    heroLevel: 0,
    hpPercent: 100,
    manaPercent: 100,
    inDanger: false,
    connected: false,
    alive: true,
    stunned: false,
    silenced: false,
    respawnTimer: null,
    runeTimer: null,
    gameTime: 0,
  },
  diagnostics: {
    gsiConnected: false,
    keyboardHookActive: false,
    queueMetrics: { eventsProcessed: 0, eventsDropped: 0, currentQueueDepth: 0, maxQueueDepth: 10 },
    syntheticInput: { queueDepth: 0, totalQueued: 0, peakDepth: 0, completions: 0, drops: 0 },
    soulRingState: "ready",
    blockedKeys: [],
  },
  updateState: { kind: "idle" },
  setGame: (partial) =>
    set((state) => ({ game: { ...state.game, ...partial } })),
}));
```

- [ ] **Step 4: Create uiStore**

Create `src-ui/src/stores/uiStore.ts`:

```ts
import { create } from "zustand";

interface UIStore {
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
  gsiEnabled: boolean;
  standaloneEnabled: boolean;
  setGsiEnabled: (enabled: boolean) => void;
  setStandaloneEnabled: (enabled: boolean) => void;
}

export const useUIStore = create<UIStore>((set) => ({
  sidebarCollapsed: false,
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  gsiEnabled: true,
  standaloneEnabled: false,
  setGsiEnabled: (enabled) => set({ gsiEnabled: enabled }),
  setStandaloneEnabled: (enabled) => set({ standaloneEnabled: enabled }),
}));
```

- [ ] **Step 5: Create activityStore**

Create `src-ui/src/stores/activityStore.ts`:

```ts
import { create } from "zustand";
import type { ActivityEntry, ActivityCategory } from "../types/activity";
import { mockActivityLog } from "./mockData";

interface ActivityStore {
  entries: ActivityEntry[];
  filter: ActivityCategory | "all";
  setFilter: (filter: ActivityCategory | "all") => void;
  addEntry: (entry: ActivityEntry) => void;
  clear: () => void;
  filteredEntries: () => ActivityEntry[];
}

export const useActivityStore = create<ActivityStore>((set, get) => ({
  entries: mockActivityLog,
  filter: "all",
  setFilter: (filter) => set({ filter }),
  addEntry: (entry) =>
    set((state) => ({ entries: [...state.entries.slice(-499), entry] })),
  clear: () => set({ entries: [] }),
  filteredEntries: () => {
    const { entries, filter } = get();
    return filter === "all" ? entries : entries.filter((e) => e.category === filter);
  },
}));
```

- [ ] **Step 6: Create stores barrel export**

Create `src-ui/src/stores/index.ts`:

```ts
export { useConfigStore } from "./configStore";
export { useGameStore } from "./gameStore";
export { useUIStore } from "./uiStore";
export { useActivityStore } from "./activityStore";
```

- [ ] **Step 7: Verify build**

```bash
cd src-ui && npm run build
```

Expected: Build succeeds.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat(ui): add Zustand stores with mock data"
```

---

## Phase 5: App Shell and Dashboard

### Task 17: App Shell with React Router

**Files:**
- Modify: `src-ui/src/App.tsx`
- Create: `src-ui/src/pages/Dashboard.tsx` (stub)
- Create: `src-ui/src/pages/Heroes.tsx` (stub)
- Create: `src-ui/src/pages/DangerDetection.tsx` (stub)
- Create: `src-ui/src/pages/SoulRing.tsx` (stub)
- Create: `src-ui/src/pages/Armlet.tsx` (stub)
- Create: `src-ui/src/pages/ActivityLog.tsx` (stub)
- Create: `src-ui/src/pages/Diagnostics.tsx` (stub)
- Create: `src-ui/src/pages/Settings.tsx` (stub)
- Create: `src-ui/src/pages/HeroDetail.tsx` (stub)

- [ ] **Step 1: Create page stubs**

Create each page file under `src-ui/src/pages/`. Every stub follows this pattern. Here is the complete code for each file:

`src-ui/src/pages/Dashboard.tsx`:
```tsx
export default function Dashboard() {
  return <div className="p-6"><h2 className="text-xl font-semibold">Dashboard</h2></div>;
}
```

`src-ui/src/pages/Heroes.tsx`:
```tsx
export default function Heroes() {
  return <div className="p-6"><h2 className="text-xl font-semibold">Heroes</h2></div>;
}
```

`src-ui/src/pages/HeroDetail.tsx`:
```tsx
export default function HeroDetail() {
  return <div className="p-6"><h2 className="text-xl font-semibold">Hero Detail</h2></div>;
}
```

`src-ui/src/pages/DangerDetection.tsx`:
```tsx
export default function DangerDetection() {
  return <div className="p-6"><h2 className="text-xl font-semibold">Danger Detection</h2></div>;
}
```

`src-ui/src/pages/SoulRing.tsx`:
```tsx
export default function SoulRing() {
  return <div className="p-6"><h2 className="text-xl font-semibold">Soul Ring</h2></div>;
}
```

`src-ui/src/pages/Armlet.tsx`:
```tsx
export default function Armlet() {
  return <div className="p-6"><h2 className="text-xl font-semibold">Armlet</h2></div>;
}
```

`src-ui/src/pages/ActivityLog.tsx`:
```tsx
export default function ActivityLog() {
  return <div className="p-6"><h2 className="text-xl font-semibold">Activity Log</h2></div>;
}
```

`src-ui/src/pages/Diagnostics.tsx`:
```tsx
export default function Diagnostics() {
  return <div className="p-6"><h2 className="text-xl font-semibold">Diagnostics</h2></div>;
}
```

`src-ui/src/pages/Settings.tsx`:
```tsx
export default function Settings() {
  return <div className="p-6"><h2 className="text-xl font-semibold">Settings</h2></div>;
}
```

- [ ] **Step 2: Wire up App.tsx with router and layout**

Replace `src-ui/src/App.tsx`:

```tsx
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { Sidebar } from "./components/layout/Sidebar";
import { StatusHeader } from "./components/layout/StatusHeader";
import { ActivityTicker } from "./components/layout/ActivityTicker";
import { useGameStore } from "./stores/gameStore";
import { useActivityStore } from "./stores/activityStore";
import Dashboard from "./pages/Dashboard";
import Heroes from "./pages/Heroes";
import HeroDetail from "./pages/HeroDetail";
import DangerDetection from "./pages/DangerDetection";
import SoulRing from "./pages/SoulRing";
import Armlet from "./pages/Armlet";
import ActivityLog from "./pages/ActivityLog";
import Diagnostics from "./pages/Diagnostics";
import Settings from "./pages/Settings";

export default function App() {
  const game = useGameStore((s) => s.game);
  const entries = useActivityStore((s) => s.entries);
  const tickerEntries = entries.slice(-3).map((e) => ({
    id: e.id,
    timestamp: e.timestamp,
    category: e.category as "action" | "danger" | "warning" | "system",
    message: e.message,
  }));

  return (
    <BrowserRouter>
      <div className="flex h-screen w-screen overflow-hidden bg-base">
        <Sidebar />
        <div className="flex flex-1 flex-col overflow-hidden">
          <StatusHeader
            heroName={game.heroName ?? undefined}
            heroLevel={game.heroLevel}
            hpPercent={game.hpPercent}
            manaPercent={game.manaPercent}
            inDanger={game.inDanger}
            connected={game.connected}
            runeTimer={game.runeTimer}
            stunned={game.stunned}
            silenced={game.silenced}
            alive={game.alive}
            respawnTimer={game.respawnTimer}
          />
          <main className="flex-1 overflow-y-auto">
            <Routes>
              <Route path="/" element={<Dashboard />} />
              <Route path="/heroes" element={<Heroes />} />
              <Route path="/heroes/:heroId" element={<HeroDetail />} />
              <Route path="/danger" element={<DangerDetection />} />
              <Route path="/soul-ring" element={<SoulRing />} />
              <Route path="/armlet" element={<Armlet />} />
              <Route path="/activity" element={<ActivityLog />} />
              <Route path="/diagnostics" element={<Diagnostics />} />
              <Route path="/settings" element={<Settings />} />
            </Routes>
          </main>
          <ActivityTicker entries={tickerEntries} />
        </div>
      </div>
    </BrowserRouter>
  );
}
```

- [ ] **Step 3: Verify build and dev server**

```bash
cd src-ui && npm run build
```

Expected: Build succeeds. Running `npm run dev` should show sidebar + status header + "Dashboard" text + activity ticker.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(ui): add app shell with routing and page stubs"
```

---

### Task 18: Dashboard Page

**Files:**
- Modify: `src-ui/src/pages/Dashboard.tsx`

- [ ] **Step 1: Implement the Dashboard page**

Replace `src-ui/src/pages/Dashboard.tsx`:

```tsx
import { Toggle } from "../components/common/Toggle";
import { Card } from "../components/common/Card";
import { useUIStore } from "../stores/uiStore";
import { useGameStore } from "../stores/gameStore";
import { useActivityStore } from "../stores/activityStore";
import { HEROES } from "../types/game";
import { Link } from "react-router-dom";

export default function Dashboard() {
  const gsiEnabled = useUIStore((s) => s.gsiEnabled);
  const setGsiEnabled = useUIStore((s) => s.setGsiEnabled);
  const standaloneEnabled = useUIStore((s) => s.standaloneEnabled);
  const setStandaloneEnabled = useUIStore((s) => s.setStandaloneEnabled);
  const heroName = useGameStore((s) => s.game.heroName);
  const entries = useActivityStore((s) => s.entries);

  const activeHero = HEROES.find(
    (h) => h.displayName.toLowerCase() === heroName?.toLowerCase(),
  );

  const recentEntries = entries.slice(-5);

  const categoryColor: Record<string, string> = {
    action: "text-terminal",
    danger: "text-danger",
    warning: "text-warning",
    system: "text-info",
    error: "text-danger",
  };

  return (
    <div className="space-y-6 p-6">
      <h2 className="text-xl font-semibold">Dashboard</h2>

      {/* Quick Controls */}
      <Card title="Quick Controls">
        <div className="space-y-3">
          <Toggle
            label="GSI Automation"
            checked={gsiEnabled}
            onChange={setGsiEnabled}
          />
          <Toggle
            label="Standalone Script"
            checked={standaloneEnabled}
            onChange={setStandaloneEnabled}
          />
        </div>
      </Card>

      {/* Active Hero */}
      <Card title="Active Hero">
        {activeHero ? (
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <span className="text-3xl">{activeHero.icon}</span>
              <div>
                <p className="font-semibold text-content">
                  {activeHero.displayName}
                </p>
                <p className="text-xs text-subtle">{activeHero.role}</p>
              </div>
            </div>
            <Link
              to={`/heroes/${activeHero.id}`}
              className="text-sm text-gold hover:underline"
            >
              View Config →
            </Link>
          </div>
        ) : (
          <div className="grid grid-cols-4 gap-2">
            <button
              type="button"
              onClick={() => useGameStore.getState().setGame({ heroName: null })}
              className="flex flex-col items-center gap-1 rounded-md border border-border p-2 text-center hover:bg-elevated transition-colors"
            >
              <span className="text-xl">🚫</span>
              <span className="text-xs text-subtle">None</span>
            </button>
            {HEROES.map((hero) => (
              <Link
                key={hero.id}
                to={`/heroes/${hero.id}`}
                className="flex flex-col items-center gap-1 rounded-md border border-border p-2 text-center hover:bg-elevated transition-colors"
              >
                <span className="text-xl">{hero.icon}</span>
                <span className="text-xs text-subtle">{hero.displayName}</span>
              </Link>
            ))}
          </div>
        )}
      </Card>

      {/* Recent Activity */}
      <Card title="Recent Activity">
        <div className="space-y-1 rounded-md bg-terminal-bg p-3 font-mono text-xs">
          {recentEntries.length === 0 ? (
            <p className="text-muted">No activity yet...</p>
          ) : (
            recentEntries.map((entry) => (
              <div key={entry.id} className="flex gap-2">
                <span className="text-muted shrink-0">{entry.timestamp}</span>
                <span className={categoryColor[entry.category] ?? "text-content"}>
                  {entry.message}
                </span>
              </div>
            ))
          )}
        </div>
        <Link
          to="/activity"
          className="mt-2 inline-block text-xs text-gold hover:underline"
        >
          View Full Log →
        </Link>
      </Card>
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

```bash
cd src-ui && npm run build
```

Expected: Build succeeds. Dashboard shows Quick Controls, Active Hero grid, and Recent Activity.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(ui): implement Dashboard page"
```

---

## Phase 6: Hero Pages

### Task 19: Heroes Grid Page

**Files:**
- Modify: `src-ui/src/pages/Heroes.tsx`

- [ ] **Step 1: Implement the Heroes grid**

Replace `src-ui/src/pages/Heroes.tsx`:

```tsx
import { Link } from "react-router-dom";
import { HEROES } from "../types/game";
import { useGameStore } from "../stores/gameStore";

export default function Heroes() {
  const heroName = useGameStore((s) => s.game.heroName);

  return (
    <div className="space-y-6 p-6">
      <h2 className="text-xl font-semibold">Heroes</h2>
      <p className="text-sm text-subtle">
        Select a hero to view and configure its automation settings.
      </p>
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
        {HEROES.map((hero) => {
          const isActive =
            heroName?.toLowerCase() === hero.displayName.toLowerCase();
          return (
            <Link
              key={hero.id}
              to={`/heroes/${hero.id}`}
              className={`flex flex-col items-center gap-3 rounded-lg border p-4 transition-colors hover:bg-elevated ${
                isActive
                  ? "border-gold bg-elevated"
                  : "border-border bg-surface"
              }`}
            >
              <span className="flex h-14 w-14 items-center justify-center rounded-full bg-base text-2xl">
                {hero.icon}
              </span>
              <div className="text-center">
                <p className="text-sm font-medium text-content">
                  {hero.displayName}
                </p>
                <p className="text-xs text-muted">{hero.role}</p>
              </div>
              {isActive && (
                <span className="rounded-full bg-gold/20 px-2 py-0.5 text-[10px] font-medium text-gold">
                  Active
                </span>
              )}
            </Link>
          );
        })}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

```bash
cd src-ui && npm run build
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(ui): implement Heroes grid page"
```

---

### Task 20: Hero Detail Shell + HeroPage Component

**Files:**
- Modify: `src-ui/src/pages/HeroDetail.tsx`
- Create: `src-ui/src/components/heroes/HeroPage.tsx`
- Create: `src-ui/src/components/heroes/configs/MeepoConfig.tsx` (placeholder import)
- Create: `src-ui/src/components/heroes/configs/index.ts`

- [ ] **Step 1: Create the shared HeroPage shell**

Create `src-ui/src/components/heroes/HeroPage.tsx`:

```tsx
import { Link } from "react-router-dom";
import { ArrowLeft } from "lucide-react";
import type { HeroInfo } from "../../types/game";

interface HeroPageProps {
  hero: HeroInfo;
  children: React.ReactNode;
}

export function HeroPage({ hero, children }: HeroPageProps) {
  return (
    <div className="space-y-6 p-6">
      <div className="flex items-center gap-4">
        <Link
          to="/heroes"
          className="flex items-center gap-1 text-sm text-subtle hover:text-content"
        >
          <ArrowLeft className="h-4 w-4" /> Heroes
        </Link>
        <span className="text-muted">/</span>
        <div className="flex items-center gap-2">
          <span className="text-2xl">{hero.icon}</span>
          <h2 className="text-xl font-semibold">{hero.displayName}</h2>
        </div>
      </div>
      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">{children}</div>
    </div>
  );
}
```

- [ ] **Step 2: Create hero config registry**

Create `src-ui/src/components/heroes/configs/index.ts`:

```ts
import type { HeroType } from "../../../types/game";
import type { ComponentType } from "react";

const configs: Record<HeroType, () => Promise<{ default: ComponentType }>> = {
  meepo: () => import("./MeepoConfig"),
  broodmother: () => import("./BroodmotherConfig"),
  huskar: () => import("./HuskarConfig"),
  largo: () => import("./LargoConfig"),
  legion_commander: () => import("./LegionCommanderConfig"),
  outworld_destroyer: () => import("./OutworldDestroyerConfig"),
  shadow_fiend: () => import("./ShadowFiendConfig"),
  tiny: () => import("./TinyConfig"),
};

export default configs;
```

- [ ] **Step 3: Update HeroDetail page to route to correct hero config**

Replace `src-ui/src/pages/HeroDetail.tsx`:

```tsx
import { useParams, Navigate } from "react-router-dom";
import { Suspense, lazy, useMemo } from "react";
import { HEROES, type HeroType } from "../types/game";
import { HeroPage } from "../components/heroes/HeroPage";
import configs from "../components/heroes/configs";

export default function HeroDetail() {
  const { heroId } = useParams<{ heroId: string }>();
  const hero = HEROES.find((h) => h.id === heroId);

  const ConfigComponent = useMemo(() => {
    if (!heroId || !(heroId in configs)) return null;
    return lazy(configs[heroId as HeroType]);
  }, [heroId]);

  if (!hero || !ConfigComponent) {
    return <Navigate to="/heroes" replace />;
  }

  return (
    <HeroPage hero={hero}>
      <Suspense
        fallback={
          <p className="col-span-2 text-subtle">Loading config...</p>
        }
      >
        <ConfigComponent />
      </Suspense>
    </HeroPage>
  );
}
```

- [ ] **Step 4: Create placeholder hero config files**

Create a minimal placeholder for each hero config. These will be replaced in subsequent tasks.

Create `src-ui/src/components/heroes/configs/MeepoConfig.tsx`:
```tsx
export default function MeepoConfig() {
  return <p className="text-subtle">Meepo config — coming soon</p>;
}
```

Create `src-ui/src/components/heroes/configs/BroodmotherConfig.tsx`:
```tsx
export default function BroodmotherConfig() {
  return <p className="text-subtle">Broodmother config — coming soon</p>;
}
```

Create `src-ui/src/components/heroes/configs/HuskarConfig.tsx`:
```tsx
export default function HuskarConfig() {
  return <p className="text-subtle">Huskar config — coming soon</p>;
}
```

Create `src-ui/src/components/heroes/configs/LargoConfig.tsx`:
```tsx
export default function LargoConfig() {
  return <p className="text-subtle">Largo config — coming soon</p>;
}
```

Create `src-ui/src/components/heroes/configs/LegionCommanderConfig.tsx`:
```tsx
export default function LegionCommanderConfig() {
  return <p className="text-subtle">Legion Commander config — coming soon</p>;
}
```

Create `src-ui/src/components/heroes/configs/OutworldDestroyerConfig.tsx`:
```tsx
export default function OutworldDestroyerConfig() {
  return <p className="text-subtle">Outworld Destroyer config — coming soon</p>;
}
```

Create `src-ui/src/components/heroes/configs/ShadowFiendConfig.tsx`:
```tsx
export default function ShadowFiendConfig() {
  return <p className="text-subtle">Shadow Fiend config — coming soon</p>;
}
```

Create `src-ui/src/components/heroes/configs/TinyConfig.tsx`:
```tsx
export default function TinyConfig() {
  return <p className="text-subtle">Tiny config — coming soon</p>;
}
```

- [ ] **Step 5: Verify build**

```bash
cd src-ui && npm run build
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(ui): add HeroPage shell and hero routing"
```

---

### Task 21: Meepo Hero Config

**Files:**
- Modify: `src-ui/src/components/heroes/configs/MeepoConfig.tsx`

The most complex hero config — serves as the pattern for all others.

- [ ] **Step 1: Implement MeepoConfig**

Replace `src-ui/src/components/heroes/configs/MeepoConfig.tsx`:

```tsx
import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { Slider } from "../../common/Slider";
import { NumberInput } from "../../common/NumberInput";
import { KeyInput } from "../../common/KeyInput";
import { TagList } from "../../common/TagList";
import { useConfigStore } from "../../../stores/configStore";

export default function MeepoConfig() {
  const config = useConfigStore((s) => s.config.heroes.meepo);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("meepo", updates);
  const setFarm = (updates: Partial<typeof config.farm_assist>) =>
    set({ farm_assist: { ...config.farm_assist, ...updates } });

  return (
    <>
      {/* Left Column */}
      <div className="space-y-4">
        <Card title="Keybindings">
          <div className="grid grid-cols-2 gap-3">
            <KeyInput label="Combo Key" value={config.standalone_key} onChange={(v) => set({ standalone_key: v })} />
            <KeyInput label="Earthbind" value={config.earthbind_key} onChange={(v) => set({ earthbind_key: v })} />
            <KeyInput label="Poof" value={config.poof_key} onChange={(v) => set({ poof_key: v })} />
            <KeyInput label="Dig" value={config.dig_key} onChange={(v) => set({ dig_key: v })} />
            <KeyInput label="MegaMeepo" value={config.megameepo_key} onChange={(v) => set({ megameepo_key: v })} />
          </div>
        </Card>

        <Card title="Combo Settings">
          <NumberInput label="Post-Blink Delay" value={config.post_blink_delay_ms} onChange={(v) => set({ post_blink_delay_ms: v })} suffix="ms" />
          <TagList label="Combo Items" items={config.combo_items} onChange={(v) => set({ combo_items: v })} />
          <div className="grid grid-cols-2 gap-3">
            <NumberInput label="Item Spam Count" value={config.combo_item_spam_count} onChange={(v) => set({ combo_item_spam_count: v })} />
            <NumberInput label="Item Delay" value={config.combo_item_delay_ms} onChange={(v) => set({ combo_item_delay_ms: v })} suffix="ms" />
            <NumberInput label="Earthbind Presses" value={config.earthbind_press_count} onChange={(v) => set({ earthbind_press_count: v })} />
            <NumberInput label="Earthbind Interval" value={config.earthbind_press_interval_ms} onChange={(v) => set({ earthbind_press_interval_ms: v })} suffix="ms" />
            <NumberInput label="Poof Presses" value={config.poof_press_count} onChange={(v) => set({ poof_press_count: v })} />
            <NumberInput label="Poof Interval" value={config.poof_press_interval_ms} onChange={(v) => set({ poof_press_interval_ms: v })} suffix="ms" />
          </div>
        </Card>
      </div>

      {/* Right Column */}
      <div className="space-y-4">
        <Card title="Danger Abilities">
          <Toggle label="Auto-Dig on Danger" checked={config.auto_dig_on_danger} onChange={(v) => set({ auto_dig_on_danger: v })} />
          <Slider label="Dig HP Threshold" value={config.dig_hp_threshold_percent} min={10} max={80} onChange={(v) => set({ dig_hp_threshold_percent: v })} suffix="%" />
          <Toggle label="Auto-MegaMeepo on Danger" checked={config.auto_megameepo_on_danger} onChange={(v) => set({ auto_megameepo_on_danger: v })} />
          <Slider label="MegaMeepo HP Threshold" value={config.megameepo_hp_threshold_percent} min={10} max={80} onChange={(v) => set({ megameepo_hp_threshold_percent: v })} suffix="%" />
          <NumberInput label="Defensive Cooldown" value={config.defensive_trigger_cooldown_ms} onChange={(v) => set({ defensive_trigger_cooldown_ms: v })} suffix="ms" />
        </Card>

        <Card title="Farm Assist" collapsible>
          <Toggle label="Enabled" checked={config.farm_assist.enabled} onChange={(v) => setFarm({ enabled: v })} />
          <KeyInput label="Toggle Key" value={config.farm_assist.toggle_key} onChange={(v) => setFarm({ toggle_key: v })} />
          <NumberInput label="Pulse Interval" value={config.farm_assist.pulse_interval_ms} onChange={(v) => setFarm({ pulse_interval_ms: v })} suffix="ms" />
          <Slider label="Min Mana" value={config.farm_assist.minimum_mana_percent} min={0} max={100} onChange={(v) => setFarm({ minimum_mana_percent: v })} suffix="%" />
          <Slider label="Min Health" value={config.farm_assist.minimum_health_percent} min={0} max={100} onChange={(v) => setFarm({ minimum_health_percent: v })} suffix="%" />
          <Toggle label="Right-Click After Poof" checked={config.farm_assist.right_click_after_poof} onChange={(v) => setFarm({ right_click_after_poof: v })} />
          <Toggle label="Suspend on Danger" checked={config.farm_assist.suspend_on_danger} onChange={(v) => setFarm({ suspend_on_danger: v })} />
          <NumberInput label="Suspend After Combo" value={config.farm_assist.suspend_after_manual_combo_ms} onChange={(v) => setFarm({ suspend_after_manual_combo_ms: v })} suffix="ms" />
        </Card>
      </div>
    </>
  );
}
```

- [ ] **Step 2: Verify build**

```bash
cd src-ui && npm run build
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(ui): implement Meepo hero config page"
```

---

### Task 22: Broodmother and Huskar Hero Configs

**Files:**
- Modify: `src-ui/src/components/heroes/configs/BroodmotherConfig.tsx`
- Modify: `src-ui/src/components/heroes/configs/HuskarConfig.tsx`

- [ ] **Step 1: Implement BroodmotherConfig**

Replace `src-ui/src/components/heroes/configs/BroodmotherConfig.tsx`:

```tsx
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
```

- [ ] **Step 2: Implement HuskarConfig**

Replace `src-ui/src/components/heroes/configs/HuskarConfig.tsx`:

```tsx
import { Card } from "../../common/Card";
import { KeyInput } from "../../common/KeyInput";
import { NumberInput } from "../../common/NumberInput";
import { useConfigStore } from "../../../stores/configStore";

export default function HuskarConfig() {
  const config = useConfigStore((s) => s.config.heroes.huskar);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("huskar", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Keybindings">
          <KeyInput label="Standalone Key" value={config.standalone_key} onChange={(v) => set({ standalone_key: v })} />
          <KeyInput label="Berserker Blood Key" value={config.berserker_blood_key} onChange={(v) => set({ berserker_blood_key: v })} />
        </Card>

        <Card title="Berserker Blood">
          <NumberInput label="Cleanse Delay" value={config.berserker_blood_delay_ms} onChange={(v) => set({ berserker_blood_delay_ms: v })} suffix="ms" />
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Armlet Override">
          <NumberInput label="Toggle Threshold" value={config.armlet_toggle_threshold} onChange={(v) => set({ armlet_toggle_threshold: v })} suffix="HP" />
          <NumberInput label="Predictive Offset" value={config.armlet_predictive_offset} onChange={(v) => set({ armlet_predictive_offset: v })} suffix="HP" />
          <NumberInput label="Toggle Cooldown" value={config.armlet_toggle_cooldown_ms} onChange={(v) => set({ armlet_toggle_cooldown_ms: v })} suffix="ms" />
        </Card>
      </div>
    </>
  );
}
```

- [ ] **Step 3: Verify build**

```bash
cd src-ui && npm run build
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(ui): implement Broodmother and Huskar hero configs"
```

---

### Task 23: Largo and Legion Commander Hero Configs

**Files:**
- Modify: `src-ui/src/components/heroes/configs/LargoConfig.tsx`
- Modify: `src-ui/src/components/heroes/configs/LegionCommanderConfig.tsx`

- [ ] **Step 1: Implement LargoConfig**

Replace `src-ui/src/components/heroes/configs/LargoConfig.tsx`:

```tsx
import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { Slider } from "../../common/Slider";
import { NumberInput } from "../../common/NumberInput";
import { KeyInput } from "../../common/KeyInput";
import { useConfigStore } from "../../../stores/configStore";

export default function LargoConfig() {
  const config = useConfigStore((s) => s.config.heroes.largo);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("largo", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Keybindings">
          <div className="grid grid-cols-2 gap-3">
            <KeyInput label="Q Ability" value={config.q_ability_key} onChange={(v) => set({ q_ability_key: v })} />
            <KeyInput label="W Ability" value={config.w_ability_key} onChange={(v) => set({ w_ability_key: v })} />
            <KeyInput label="E Ability" value={config.e_ability_key} onChange={(v) => set({ e_ability_key: v })} />
            <KeyInput label="R Ability" value={config.r_ability_key} onChange={(v) => set({ r_ability_key: v })} />
          </div>
        </Card>

        <Card title="Amphibian Rhapsody">
          <Toggle label="Enable" checked={config.amphibian_rhapsody_enabled} onChange={(v) => set({ amphibian_rhapsody_enabled: v })} />
          <NumberInput label="Beat Interval" value={config.beat_interval_ms} onChange={(v) => set({ beat_interval_ms: v })} suffix="ms" />
          <NumberInput label="Beat Correction" value={config.beat_correction_ms} onChange={(v) => set({ beat_correction_ms: v })} suffix="ms" />
          <NumberInput label="Correct Every N Beats" value={config.beat_correction_every_n_beats} onChange={(v) => set({ beat_correction_every_n_beats: v })} />
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Auto Behavior">
          <Toggle label="Auto Toggle on Danger" checked={config.auto_toggle_on_danger} onChange={(v) => set({ auto_toggle_on_danger: v })} />
          <Slider label="Mana Threshold" value={config.mana_threshold_percent} min={0} max={100} onChange={(v) => set({ mana_threshold_percent: v })} suffix="%" />
          <Slider label="Heal HP Threshold" value={config.heal_hp_threshold} min={0} max={100} onChange={(v) => set({ heal_hp_threshold: v })} suffix="%" />
        </Card>
      </div>
    </>
  );
}
```

- [ ] **Step 2: Implement LegionCommanderConfig**

Replace `src-ui/src/components/heroes/configs/LegionCommanderConfig.tsx`:

```tsx
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
```

- [ ] **Step 3: Verify build**

```bash
cd src-ui && npm run build
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(ui): implement Largo and Legion Commander hero configs"
```

---

### Task 24: OD, Shadow Fiend, and Tiny Hero Configs

**Files:**
- Modify: `src-ui/src/components/heroes/configs/OutworldDestroyerConfig.tsx`
- Modify: `src-ui/src/components/heroes/configs/ShadowFiendConfig.tsx`
- Modify: `src-ui/src/components/heroes/configs/TinyConfig.tsx`

- [ ] **Step 1: Implement OutworldDestroyerConfig**

Replace `src-ui/src/components/heroes/configs/OutworldDestroyerConfig.tsx`:

```tsx
import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { Slider } from "../../common/Slider";
import { NumberInput } from "../../common/NumberInput";
import { KeyInput } from "../../common/KeyInput";
import { TagList } from "../../common/TagList";
import { useConfigStore } from "../../../stores/configStore";

export default function OutworldDestroyerConfig() {
  const config = useConfigStore((s) => s.config.heroes.outworld_destroyer);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("outworld_destroyer", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Keybindings">
          <div className="grid grid-cols-2 gap-3">
            <KeyInput label="Combo Key" value={config.standalone_key} onChange={(v) => set({ standalone_key: v })} />
            <KeyInput label="Objurgation" value={config.objurgation_key} onChange={(v) => set({ objurgation_key: v })} />
            <KeyInput label="Arcane Orb" value={config.arcane_orb_key} onChange={(v) => set({ arcane_orb_key: v })} />
            <KeyInput label="Astral Imprisonment" value={config.astral_imprisonment_key} onChange={(v) => set({ astral_imprisonment_key: v })} />
          </div>
        </Card>

        <Card title="Auto-Objurgation on Danger">
          <Toggle label="Enable" checked={config.auto_objurgation_on_danger} onChange={(v) => set({ auto_objurgation_on_danger: v })} />
          <Slider label="HP Threshold" value={config.objurgation_hp_threshold_percent} min={10} max={90} onChange={(v) => set({ objurgation_hp_threshold_percent: v })} suffix="%" />
          <Slider label="Min Mana" value={config.objurgation_min_mana_percent} min={0} max={100} onChange={(v) => set({ objurgation_min_mana_percent: v })} suffix="%" />
          <NumberInput label="Trigger Cooldown" value={config.objurgation_trigger_cooldown_ms} onChange={(v) => set({ objurgation_trigger_cooldown_ms: v })} suffix="ms" />
        </Card>

        <Card title="Ultimate Intercept">
          <Toggle label="Enable" checked={config.ultimate_intercept_enabled} onChange={(v) => set({ ultimate_intercept_enabled: v })} />
          <Toggle label="Auto-BKB on Ultimate" checked={config.auto_bkb_on_ultimate} onChange={(v) => set({ auto_bkb_on_ultimate: v })} />
          <Toggle label="Auto-Objurgation on Ultimate" checked={config.auto_objurgation_on_ultimate} onChange={(v) => set({ auto_objurgation_on_ultimate: v })} />
          <NumberInput label="Post-BKB Delay" value={config.post_bkb_delay_ms} onChange={(v) => set({ post_bkb_delay_ms: v })} suffix="ms" />
          <NumberInput label="Post-Blink Delay" value={config.post_blink_delay_ms} onChange={(v) => set({ post_blink_delay_ms: v })} suffix="ms" />
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Standalone Combo">
          <TagList label="Combo Items" items={config.combo_items} onChange={(v) => set({ combo_items: v })} />
          <div className="grid grid-cols-2 gap-3">
            <NumberInput label="Item Spam Count" value={config.combo_item_spam_count} onChange={(v) => set({ combo_item_spam_count: v })} />
            <NumberInput label="Item Delay" value={config.combo_item_delay_ms} onChange={(v) => set({ combo_item_delay_ms: v })} suffix="ms" />
            <NumberInput label="Post-Ult Orb Presses" value={config.post_ultimate_arcane_orb_presses} onChange={(v) => set({ post_ultimate_arcane_orb_presses: v })} />
            <NumberInput label="Orb Press Interval" value={config.arcane_orb_press_interval_ms} onChange={(v) => set({ arcane_orb_press_interval_ms: v })} suffix="ms" />
          </div>
        </Card>

        <Card title="Self-Astral Panic" collapsible>
          <Toggle label="Enable" checked={config.astral_self_cast_enabled} onChange={(v) => set({ astral_self_cast_enabled: v })} />
          <KeyInput label="Panic Key" value={config.astral_self_cast_key} onChange={(v) => set({ astral_self_cast_key: v })} />
        </Card>
      </div>
    </>
  );
}
```

- [ ] **Step 2: Implement ShadowFiendConfig**

Replace `src-ui/src/components/heroes/configs/ShadowFiendConfig.tsx`:

```tsx
import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { NumberInput } from "../../common/NumberInput";
import { useConfigStore } from "../../../stores/configStore";

export default function ShadowFiendConfig() {
  const config = useConfigStore((s) => s.config.heroes.shadow_fiend);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("shadow_fiend", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Raze Intercept">
          <Toggle label="Enable Raze Intercept" checked={config.raze_intercept_enabled} onChange={(v) => set({ raze_intercept_enabled: v })} />
          <NumberInput label="Raze Delay" value={config.raze_delay_ms} onChange={(v) => set({ raze_delay_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            Intercepts Q/W/E to face cursor direction before razing.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Ultimate Intercept">
          <Toggle label="Auto-BKB on Ultimate" checked={config.auto_bkb_on_ultimate} onChange={(v) => set({ auto_bkb_on_ultimate: v })} />
          <Toggle label="Auto-D Ability on Ultimate" checked={config.auto_d_on_ultimate} onChange={(v) => set({ auto_d_on_ultimate: v })} />
        </Card>
      </div>
    </>
  );
}
```

- [ ] **Step 3: Implement TinyConfig**

Replace `src-ui/src/components/heroes/configs/TinyConfig.tsx`:

```tsx
import { Card } from "../../common/Card";
import { KeyInput } from "../../common/KeyInput";
import { useConfigStore } from "../../../stores/configStore";

export default function TinyConfig() {
  const config = useConfigStore((s) => s.config.heroes.tiny);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("tiny", updates);

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
              {["Blink", "Avalanche (W + Soul Ring)", "W ×3", "Toss (Q) ×4", "Tree Grab (D) ×3"].map((step, i) => (
                <span key={i} className="rounded bg-elevated px-2 py-0.5 font-mono">
                  {i > 0 && "→ "}{step}
                </span>
              ))}
            </div>
            <p className="mt-2 text-muted">Soul Ring is automatically used before Avalanche if available.</p>
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
```

- [ ] **Step 4: Verify build**

```bash
cd src-ui && npm run build
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(ui): implement OD, Shadow Fiend, and Tiny hero configs"
```

---

## Phase 7: Feature Pages

### Task 25: Danger Detection Page

**Files:**
- Modify: `src-ui/src/pages/DangerDetection.tsx`

- [ ] **Step 1: Implement DangerDetection page**

Replace `src-ui/src/pages/DangerDetection.tsx`:

```tsx
import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { Slider } from "../components/common/Slider";
import { KeyInput } from "../components/common/KeyInput";
import { TagList } from "../components/common/TagList";
import { useConfigStore } from "../stores/configStore";

export default function DangerDetection() {
  const danger = useConfigStore((s) => s.config.danger_detection);
  const neutral = useConfigStore((s) => s.config.neutral_items);
  const updateDanger = (updates: Partial<typeof danger>) =>
    useConfigStore.getState().updateConfig("danger_detection", updates);
  const updateNeutral = (updates: Partial<typeof neutral>) =>
    useConfigStore.getState().updateConfig("neutral_items", updates);

  return (
    <div className="space-y-6 p-6">
      <h2 className="text-xl font-semibold">Danger Detection</h2>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="Core Settings">
            <Toggle label="Enable Danger Detection" checked={danger.enabled} onChange={(v) => updateDanger({ enabled: v })} />
            <Slider label="HP Threshold" value={danger.hp_threshold_percent} min={30} max={90} onChange={(v) => updateDanger({ hp_threshold_percent: v })} suffix="%" />
            <Slider label="Rapid Loss Threshold" value={danger.rapid_loss_hp} min={50} max={300} onChange={(v) => updateDanger({ rapid_loss_hp: v })} suffix=" HP" />
            <Slider label="Burst Time Window" value={danger.time_window_ms} min={100} max={2000} onChange={(v) => updateDanger({ time_window_ms: v })} suffix="ms" />
            <Slider label="Clear Delay" value={danger.clear_delay_seconds} min={1} max={10} onChange={(v) => updateDanger({ clear_delay_seconds: v })} suffix="s" />
          </Card>

          <Card title="Healing in Danger">
            <Slider label="Healing HP Threshold" value={danger.healing_threshold_in_danger} min={30} max={80} onChange={(v) => updateDanger({ healing_threshold_in_danger: v })} suffix="%" />
            <Slider label="Max Healing Items/Event" value={danger.max_healing_items_per_danger} min={1} max={5} onChange={(v) => updateDanger({ max_healing_items_per_danger: v })} />
            <div className="mt-2 text-xs text-muted">
              <p className="font-medium text-subtle">Priority: Cheese → Greater Faerie Fire → Enchanted Mango → Magic Wand → Faerie Fire</p>
            </div>
          </Card>
        </div>

        <div className="space-y-4">
          <Card title="Defensive Items">
            <Toggle label="Black King Bar" checked={danger.auto_bkb} onChange={(v) => updateDanger({ auto_bkb: v })} />
            <Toggle label="Satanic" checked={danger.auto_satanic} onChange={(v) => updateDanger({ auto_satanic: v })} />
            {danger.auto_satanic && (
              <Slider label="Satanic HP Threshold" value={danger.satanic_hp_threshold} min={10} max={70} onChange={(v) => updateDanger({ satanic_hp_threshold: v })} suffix="%" />
            )}
            <Toggle label="Blade Mail" checked={danger.auto_blade_mail} onChange={(v) => updateDanger({ auto_blade_mail: v })} />
            <Toggle label="Glimmer Cape" checked={danger.auto_glimmer_cape} onChange={(v) => updateDanger({ auto_glimmer_cape: v })} />
            <Toggle label="Ghost Scepter" checked={danger.auto_ghost_scepter} onChange={(v) => updateDanger({ auto_ghost_scepter: v })} />
            <Toggle label="Shiva's Guard" checked={danger.auto_shivas_guard} onChange={(v) => updateDanger({ auto_shivas_guard: v })} />
          </Card>

          <Card title="Dispels">
            <Toggle label="Auto-Manta on Silence" checked={danger.auto_manta_on_silence} onChange={(v) => updateDanger({ auto_manta_on_silence: v })} />
            <Toggle label="Auto-Lotus on Silence" checked={danger.auto_lotus_on_silence} onChange={(v) => updateDanger({ auto_lotus_on_silence: v })} />
          </Card>

          <Card title="Neutral Items" collapsible>
            <Toggle label="Enable" checked={neutral.enabled} onChange={(v) => updateNeutral({ enabled: v })} />
            <Toggle label="Use in Danger Only" checked={neutral.use_in_danger} onChange={(v) => updateNeutral({ use_in_danger: v })} />
            <Slider label="HP Threshold" value={neutral.hp_threshold} min={10} max={90} onChange={(v) => updateNeutral({ hp_threshold: v })} suffix="%" />
            <KeyInput label="Self-Cast Key" value={neutral.self_cast_key} onChange={(v) => updateNeutral({ self_cast_key: v })} />
            <TagList label="Allowed Items" items={neutral.allowed_items} onChange={(v) => updateNeutral({ allowed_items: v })} />
          </Card>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

```bash
cd src-ui && npm run build
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(ui): implement Danger Detection page"
```

---

### Task 26: Soul Ring Page

**Files:**
- Modify: `src-ui/src/pages/SoulRing.tsx`

- [ ] **Step 1: Implement SoulRing page**

Replace `src-ui/src/pages/SoulRing.tsx`:

```tsx
import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { Slider } from "../components/common/Slider";
import { NumberInput } from "../components/common/NumberInput";
import { useConfigStore } from "../stores/configStore";

export default function SoulRing() {
  const config = useConfigStore((s) => s.config.soul_ring);
  const update = (updates: Partial<typeof config>) =>
    useConfigStore.getState().updateConfig("soul_ring", updates);

  return (
    <div className="space-y-6 p-6">
      <h2 className="text-xl font-semibold">Soul Ring</h2>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="Settings">
            <Toggle label="Enable Soul Ring" checked={config.enabled} onChange={(v) => update({ enabled: v })} />
            <Slider label="Min Mana to Trigger" value={config.min_mana_percent} min={0} max={100} onChange={(v) => update({ min_mana_percent: v })} suffix="%" />
            <Slider label="Min Health Safety Floor" value={config.min_health_percent} min={0} max={50} onChange={(v) => update({ min_health_percent: v })} suffix="%" />
            <NumberInput label="Delay Before Ability" value={config.delay_before_ability_ms} onChange={(v) => update({ delay_before_ability_ms: v })} suffix="ms" />
            <NumberInput label="Trigger Cooldown" value={config.trigger_cooldown_ms} onChange={(v) => update({ trigger_cooldown_ms: v })} suffix="ms" />
          </Card>
        </div>

        <div className="space-y-4">
          <Card title="Intercepted Keys">
            <div className="flex flex-wrap gap-2">
              {config.ability_keys.map((key) => (
                <span
                  key={key}
                  className="flex h-8 w-8 items-center justify-center rounded-md border border-border bg-elevated font-mono text-sm font-semibold text-gold"
                >
                  {key.toUpperCase()}
                </span>
              ))}
            </div>
            <Toggle
              label="Intercept Item Keys"
              checked={config.intercept_item_keys}
              onChange={(v) => update({ intercept_item_keys: v })}
            />
            <p className="text-xs text-muted">
              Soul Ring pre-casts before these keys when mana is below threshold.
              Excludes Blink, TP, BKB, Armlet, and consumables.
            </p>
          </Card>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

```bash
cd src-ui && npm run build
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(ui): implement Soul Ring page"
```

---

### Task 27: Armlet Page

**Files:**
- Modify: `src-ui/src/pages/Armlet.tsx`

- [ ] **Step 1: Implement Armlet page**

Replace `src-ui/src/pages/Armlet.tsx`:

```tsx
import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { NumberInput } from "../components/common/NumberInput";
import { Dropdown } from "../components/common/Dropdown";
import { useConfigStore } from "../stores/configStore";
import { HEROES } from "../types/game";
import { Link } from "react-router-dom";

export default function Armlet() {
  const config = useConfigStore((s) => s.config.armlet);
  const heroes = useConfigStore((s) => s.config.heroes);
  const update = (updates: Partial<typeof config>) =>
    useConfigStore.getState().updateConfig("armlet", updates);

  const heroesWithOverrides = HEROES.filter((h) => {
    const heroConfig = heroes[h.id as keyof typeof heroes];
    return heroConfig && "armlet" in heroConfig;
  });

  return (
    <div className="space-y-6 p-6">
      <h2 className="text-xl font-semibold">Armlet</h2>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="Shared Settings">
            <Toggle label="Enable Armlet" checked={config.enabled} onChange={(v) => update({ enabled: v })} />
            <Dropdown
              label="Cast Modifier"
              value={config.cast_modifier}
              options={[
                { value: "Alt", label: "Alt" },
                { value: "Ctrl", label: "Ctrl" },
                { value: "Shift", label: "Shift" },
              ]}
              onChange={(v) => update({ cast_modifier: v })}
            />
            <NumberInput label="Toggle Threshold" value={config.toggle_threshold} onChange={(v) => update({ toggle_threshold: v })} suffix="HP" />
            <NumberInput label="Predictive Offset" value={config.predictive_offset} onChange={(v) => update({ predictive_offset: v })} suffix="HP" />
            <NumberInput label="Toggle Cooldown" value={config.toggle_cooldown_ms} onChange={(v) => update({ toggle_cooldown_ms: v })} suffix="ms" />
          </Card>
        </div>

        <div className="space-y-4">
          <Card title="Per-Hero Overrides">
            <div className="space-y-2">
              {heroesWithOverrides.map((hero) => (
                <Link
                  key={hero.id}
                  to={`/heroes/${hero.id}`}
                  className="flex items-center justify-between rounded-md border border-border bg-base p-3 transition-colors hover:bg-elevated"
                >
                  <div className="flex items-center gap-2">
                    <span className="text-lg">{hero.icon}</span>
                    <span className="text-sm text-content">{hero.displayName}</span>
                  </div>
                  <span className="text-xs text-gold">Configure →</span>
                </Link>
              ))}
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

```bash
cd src-ui && npm run build
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(ui): implement Armlet page"
```

---

## Phase 8: Utility Pages

### Task 28: Activity Log Page

**Files:**
- Modify: `src-ui/src/pages/ActivityLog.tsx`

- [ ] **Step 1: Implement ActivityLog page**

Replace `src-ui/src/pages/ActivityLog.tsx`:

```tsx
import { useRef, useEffect, useState } from "react";
import { useActivityStore } from "../stores/activityStore";
import { Button } from "../components/common/Button";
import type { ActivityCategory } from "../types/activity";

const filters: { label: string; value: ActivityCategory | "all" }[] = [
  { label: "All", value: "all" },
  { label: "Actions", value: "action" },
  { label: "Danger", value: "danger" },
  { label: "Errors", value: "error" },
  { label: "System", value: "system" },
];

const categoryColors: Record<string, string> = {
  action: "text-terminal",
  danger: "text-danger",
  warning: "text-warning",
  system: "text-info",
  error: "text-danger",
};

export default function ActivityLog() {
  const entries = useActivityStore((s) => s.filteredEntries());
  const filter = useActivityStore((s) => s.filter);
  const setFilter = useActivityStore((s) => s.setFilter);
  const clear = useActivityStore((s) => s.clear);
  const [paused, setPaused] = useState(false);
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!paused) {
      endRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [entries.length, paused]);

  return (
    <div className="flex h-full flex-col p-6">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-xl font-semibold">Activity Log</h2>
        <div className="flex items-center gap-2">
          <Button
            variant="secondary"
            onClick={() => setPaused(!paused)}
          >
            {paused ? "Resume" : "Pause"}
          </Button>
          <Button variant="danger" onClick={clear}>
            Clear
          </Button>
        </div>
      </div>

      <div className="mb-4 flex gap-2">
        {filters.map((f) => (
          <button
            key={f.value}
            type="button"
            onClick={() => setFilter(f.value)}
            className={`rounded-full px-3 py-1 text-xs font-medium transition-colors ${
              filter === f.value
                ? "bg-gold text-base"
                : "bg-elevated text-subtle hover:text-content"
            }`}
          >
            {f.label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto rounded-lg bg-terminal-bg p-4 font-mono text-xs">
        {entries.length === 0 ? (
          <p className="text-muted">No activity entries.</p>
        ) : (
          <div className="space-y-0.5">
            {entries.map((entry) => (
              <div key={entry.id} className="flex gap-3">
                <span className="shrink-0 text-muted">&gt; {entry.timestamp}</span>
                <span className={`shrink-0 w-16 uppercase ${categoryColors[entry.category]}`}>
                  [{entry.category}]
                </span>
                <span className={categoryColors[entry.category]}>
                  {entry.message}
                </span>
              </div>
            ))}
            <div ref={endRef} />
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

```bash
cd src-ui && npm run build
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(ui): implement Activity Log page"
```

---

### Task 29: Diagnostics Page

**Files:**
- Modify: `src-ui/src/pages/Diagnostics.tsx`

- [ ] **Step 1: Implement Diagnostics page**

Replace `src-ui/src/pages/Diagnostics.tsx`:

```tsx
import { Card } from "../components/common/Card";
import { useGameStore } from "../stores/gameStore";

function StatusDot({ active, label }: { active: boolean; label: string }) {
  return (
    <div className="flex items-center justify-between rounded-md border border-border bg-base p-3">
      <span className="text-sm text-content">{label}</span>
      <div className="flex items-center gap-2">
        <span className={`h-2.5 w-2.5 rounded-full ${active ? "bg-success" : "bg-danger"}`} />
        <span className={`text-xs font-mono ${active ? "text-success" : "text-danger"}`}>
          {active ? "Active" : "Inactive"}
        </span>
      </div>
    </div>
  );
}

function MetricRow({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="flex items-center justify-between py-1">
      <span className="text-xs text-subtle">{label}</span>
      <span className="font-mono text-xs text-content">{value}</span>
    </div>
  );
}

export default function Diagnostics() {
  const diag = useGameStore((s) => s.diagnostics);

  return (
    <div className="space-y-6 p-6">
      <h2 className="text-xl font-semibold">Diagnostics</h2>

      <div className="grid grid-cols-3 gap-4">
        <StatusDot active={diag.gsiConnected} label="GSI Server" />
        <StatusDot active={diag.keyboardHookActive} label="Keyboard Hook" />
        <StatusDot active={diag.gsiConnected} label="Game State" />
      </div>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="GSI Pipeline">
            <MetricRow label="Events Processed" value={diag.queueMetrics.eventsProcessed} />
            <MetricRow label="Events Dropped" value={diag.queueMetrics.eventsDropped} />
            <MetricRow label="Queue Depth" value={`${diag.queueMetrics.currentQueueDepth} / ${diag.queueMetrics.maxQueueDepth}`} />
          </Card>

          <Card title="Keyboard Hook">
            <MetricRow label="Soul Ring State" value={diag.soulRingState} />
            <MetricRow label="Blocked Keys" value={diag.blockedKeys.join(", ") || "None"} />
          </Card>
        </div>

        <div className="space-y-4">
          <Card title="Synthetic Input">
            <MetricRow label="Queue Depth" value={diag.syntheticInput.queueDepth} />
            <MetricRow label="Total Queued" value={diag.syntheticInput.totalQueued} />
            <MetricRow label="Peak Depth" value={diag.syntheticInput.peakDepth} />
            <MetricRow label="Completions" value={diag.syntheticInput.completions} />
            <MetricRow label="Drops" value={diag.syntheticInput.drops} />
          </Card>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

```bash
cd src-ui && npm run build
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(ui): implement Diagnostics page"
```

---

### Task 30: Settings Page

**Files:**
- Modify: `src-ui/src/pages/Settings.tsx`

- [ ] **Step 1: Implement Settings page**

Replace `src-ui/src/pages/Settings.tsx`:

```tsx
import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { Slider } from "../components/common/Slider";
import { NumberInput } from "../components/common/NumberInput";
import { KeyInput } from "../components/common/KeyInput";
import { Dropdown } from "../components/common/Dropdown";
import { Button } from "../components/common/Button";
import { useConfigStore } from "../stores/configStore";

export default function Settings() {
  const config = useConfigStore((s) => s.config);
  const updateConfig = useConfigStore((s) => s.updateConfig);

  return (
    <div className="space-y-6 p-6">
      <h2 className="text-xl font-semibold">Settings</h2>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          <Card title="Server">
            <NumberInput
              label="GSI Port"
              value={config.server.port}
              onChange={(v) => updateConfig("server", { port: v })}
            />
            <p className="text-xs text-warning">⚠ Restart required after changing port.</p>
          </Card>

          <Card title="Keybindings">
            <div className="grid grid-cols-3 gap-3">
              <KeyInput label="Slot 1" value={config.keybindings.slot0} onChange={(v) => updateConfig("keybindings", { slot0: v })} />
              <KeyInput label="Slot 2" value={config.keybindings.slot1} onChange={(v) => updateConfig("keybindings", { slot1: v })} />
              <KeyInput label="Slot 3" value={config.keybindings.slot2} onChange={(v) => updateConfig("keybindings", { slot2: v })} />
              <KeyInput label="Slot 4" value={config.keybindings.slot3} onChange={(v) => updateConfig("keybindings", { slot3: v })} />
              <KeyInput label="Slot 5" value={config.keybindings.slot4} onChange={(v) => updateConfig("keybindings", { slot4: v })} />
              <KeyInput label="Slot 6" value={config.keybindings.slot5} onChange={(v) => updateConfig("keybindings", { slot5: v })} />
            </div>
            <KeyInput label="Neutral Slot" value={config.keybindings.neutral0} onChange={(v) => updateConfig("keybindings", { neutral0: v })} />
            <KeyInput label="Combo Trigger" value={config.keybindings.combo_trigger} onChange={(v) => updateConfig("keybindings", { combo_trigger: v })} />
          </Card>

          <Card title="Common">
            <NumberInput
              label="Survivability HP Threshold"
              value={config.common.survivability_hp_threshold}
              onChange={(v) => updateConfig("common", { survivability_hp_threshold: v })}
              suffix="%"
            />
          </Card>
        </div>

        <div className="space-y-4">
          <Card title="Rune Alerts">
            <Toggle label="Enable Rune Alerts" checked={config.rune_alerts.enabled} onChange={(v) => updateConfig("rune_alerts", { enabled: v })} />
            <NumberInput label="Alert Lead Time" value={config.rune_alerts.alert_lead_seconds} onChange={(v) => updateConfig("rune_alerts", { alert_lead_seconds: v })} suffix="s" />
            <NumberInput label="Check Interval" value={config.rune_alerts.interval_seconds} onChange={(v) => updateConfig("rune_alerts", { interval_seconds: v })} suffix="s" />
            <Toggle label="Audio Alert" checked={config.rune_alerts.audio_enabled} onChange={(v) => updateConfig("rune_alerts", { audio_enabled: v })} />
          </Card>

          <Card title="Application">
            <Toggle label="Check for Updates on Startup" checked={config.updates.check_on_startup} onChange={(v) => updateConfig("updates", { check_on_startup: v })} />
            <Toggle label="Include Pre-releases" checked={config.updates.include_prereleases} onChange={(v) => updateConfig("updates", { include_prereleases: v })} />
            <Dropdown
              label="Log Level"
              value={config.logging.level}
              options={[
                { value: "debug", label: "Debug" },
                { value: "info", label: "Info" },
                { value: "warn", label: "Warn" },
                { value: "error", label: "Error" },
              ]}
              onChange={(v) => updateConfig("logging", { level: v as "debug" | "info" | "warn" | "error" })}
            />
          </Card>

          <Card title="Advanced" collapsible defaultOpen={false}>
            <Toggle label="Enable Minimap Capture (Experimental)" checked={config.minimap_capture.enabled} onChange={(v) => updateConfig("minimap_capture", { enabled: v })} />
            {config.minimap_capture.enabled && (
              <div className="grid grid-cols-2 gap-3">
                <NumberInput label="X" value={config.minimap_capture.minimap_x} onChange={(v) => updateConfig("minimap_capture", { minimap_x: v })} />
                <NumberInput label="Y" value={config.minimap_capture.minimap_y} onChange={(v) => updateConfig("minimap_capture", { minimap_y: v })} />
                <NumberInput label="Width" value={config.minimap_capture.minimap_width} onChange={(v) => updateConfig("minimap_capture", { minimap_width: v })} />
                <NumberInput label="Height" value={config.minimap_capture.minimap_height} onChange={(v) => updateConfig("minimap_capture", { minimap_height: v })} />
                <NumberInput label="Capture Interval" value={config.minimap_capture.capture_interval_ms} onChange={(v) => updateConfig("minimap_capture", { capture_interval_ms: v })} suffix="ms" />
                <NumberInput label="Sample Every N" value={config.minimap_capture.sample_every_n} onChange={(v) => updateConfig("minimap_capture", { sample_every_n: v })} />
              </div>
            )}
          </Card>

          <Button variant="danger" className="w-full">
            Reset to Defaults
          </Button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

```bash
cd src-ui && npm run build
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(ui): implement Settings page"
```

---

## Final Verification

### Task 31: Full Build + Test Suite

- [ ] **Step 1: Run all tests**

```bash
cd src-ui && npm test
```

Expected: All component tests pass.

- [ ] **Step 2: Run production build**

```bash
cd src-ui && npm run build
```

Expected: Build succeeds with no errors.

- [ ] **Step 3: Verify dev server renders all pages**

```bash
cd src-ui && npm run dev
```

Manually check in browser:
- `/` — Dashboard with toggles, hero grid, activity feed
- `/heroes` — 2×4 hero grid
- `/heroes/meepo` — Full Meepo config with all sections
- `/heroes/shadow_fiend` — SF raze intercept + ultimate settings
- `/danger` — Danger detection with sliders, toggles, neutral items
- `/soul-ring` — Soul Ring settings with key badges
- `/armlet` — Armlet shared settings + hero override links
- `/activity` — Filter pills + terminal log
- `/diagnostics` — Status dots + metric cards
- `/settings` — Server, keybindings, rune alerts, app config, advanced section

- [ ] **Step 4: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix(ui): final adjustments after full verification"
```

---

## Summary

| Phase | Tasks | What's Built |
|-------|-------|-------------|
| 1. Foundation | 1–3 | Vite + React scaffold, Tailwind v4 design tokens |
| 2. Components | 4–11 | Toggle, Card, Button, Slider, NumberInput, KeyInput, TagList, HPBar, ManaBar, DangerBadge, Dropdown |
| 3. Layout | 12–14 | Sidebar, StatusHeader, ActivityTicker |
| 4. Data Layer | 15–16 | TypeScript types, Zustand stores with mock data |
| 5. App Shell | 17–18 | React Router, page stubs, Dashboard |
| 6. Hero Pages | 19–24 | Heroes grid, HeroPage shell, all 8 hero configs |
| 7. Feature Pages | 25–27 | Danger Detection, Soul Ring, Armlet |
| 8. Utility Pages | 28–30 | Activity Log, Diagnostics, Settings |
| Verification | 31 | Full build + test + manual verification |

**Total:** 31 tasks, ~50 files created

**Next plan:** Tauri v2 integration — wrapping this React app in Tauri, adding Rust command handlers, IPC hooks, and replacing mock data with real backend state.

## Deferred to Plan 2: Tauri Integration

The following spec requirements depend on Tauri IPC or real-time backend state and are intentionally deferred:

- **Update Banner** — Requires Tauri commands to check/download/apply updates
- **Sidebar collapse/expand** — Responsive behavior enhancement (60px collapsed, hover-expand)
- **Live Status sections on hero pages** — Real-time ability cooldowns, combo state from game
- **Connection state banners** — "Connection to backend lost" banner, toast on config write failure
- **Config validation** — Inline errors, range enforcement, key conflict detection
- **`useTauriCommand` / `useTauriEvent` hooks** — Tauri-specific data hooks
- **Settings "Check for Updates Now" button** — Triggers Tauri update check
- **`updateStore` as separate store** — Will be split from gameStore when Tauri events are wired
- **Page transition animations** — 150ms fade transitions (CSS-only, can be added anytime)
- **Activity Log entry expand on click** — Detail expansion (minor UX polish)
