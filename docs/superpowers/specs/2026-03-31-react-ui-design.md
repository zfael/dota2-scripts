# React UI/UX Design Spec — Dota 2 Scripts

**Date:** 2026-03-31
**Status:** Approved
**Scope:** Replace the existing egui frontend with a React.js UI delivered via Tauri, maintaining the Rust backend.

---

## 1. Overview

The Dota 2 Scripts application currently uses egui/eframe for its desktop GUI. This spec defines a React.js replacement delivered via Tauri, providing a modern dark gaming aesthetic with terminal/hacker accents, full in-app configuration editing, real-time game state display, and per-hero dedicated pages.

### Delivery

- **Framework:** Tauri v2 (Rust backend stays, React frontend replaces egui)
- **Frontend:** React.js with TypeScript
- **IPC:** Tauri commands bridge Rust backend ↔ React frontend
- **Window:** Flexible/resizable, minimum 900×650

### Design Principles

- Dark gaming aesthetic inspired by Dota 2, with subtle terminal/hacker accents
- Sidebar navigation for scalability
- Full configuration editing through the UI (writes back to `config/config.toml`)
- Two app modes: idle (pre-game) and in-game (GSI active)
- Activity log for real-time visibility into automation actions

---

## 2. Layout Architecture

### Overall Structure

```
┌──────────────────────────────────────────────────────────────────┐
│                     STATUS HEADER BAR                            │
├──────────┬───────────────────────────────────────────────────────┤
│          │                                                       │
│ SIDEBAR  │              ACTIVE PAGE CONTENT                      │
│   NAV    │                                                       │
│          │                                                       │
│          │                                                       │
│          │                                                       │
│          │                                                       │
├──────────┴───────────────────────────────────────────────────────┤
│                   ACTIVITY TICKER (collapsible)                  │
└──────────────────────────────────────────────────────────────────┘
```

### Status Header Bar

Persistent horizontal bar across the top of the content area (right of sidebar). Always visible.

**Idle state (no GSI data):**
- App title: "D2 Scripts"
- Pulsing dot: "Waiting for game..."
- App version on right

**In-game state (GSI active):**
- Hero name + level (auto-detected)
- HP bar: gradient green→yellow→red, numerical percentage overlaid
- Mana bar: blue, numerical percentage overlaid
- Contextual status icons: ⚡ Stunned, 🔇 Silenced, 💀 Dead (with respawn timer)
- Danger indicator: red pulsing "⚠ DANGER" badge (only when danger state active)
- Rune timer: countdown to next rune spawn, flashes amber when imminent
- Connection indicator: green dot = GSI connected, red = disconnected

**Update Banner:**
Appears as a persistent banner below the status header when an update is available. Visible across all pages.

| State | Display |
|-------|---------|
| Checking | Spinner + "Checking for updates..." |
| Available | Green banner: version + collapsible release notes + "Update Now" / "Later" buttons |
| Downloading | Spinner + "Downloading update..." |
| Error | Red banner: error message + "Retry" / "Dismiss" buttons |
| UpToDate / Idle | Hidden |

### Sidebar Navigation

Always visible on the left side. Collapses to ~60px (icons only) at narrow widths, expands to ~200px (icons + labels) at wider widths. Hover temporarily expands when collapsed.

**Navigation items (top to bottom):**

| Icon | Label | Route | Purpose |
|------|-------|-------|---------|
| 🏠 | Dashboard | `/` | Home — overview, quick toggles, recent activity |
| ⚔️ | Heroes | `/heroes` | Hero selector grid → `/heroes/:name` for individual hero pages |
| 🛡️ | Danger | `/danger` | Danger detection settings & defensive items |
| 💍 | Soul Ring | `/soul-ring` | Soul Ring automation config |
| 🗡️ | Armlet | `/armlet` | Armlet toggle config (shared + per-hero overrides) |
| 📋 | Activity | `/activity` | Real-time scrolling activity log |
| 🔧 | Diagnostics | `/diagnostics` | Debug metrics, GSI queue stats, synthetic input stats |
| ⚙️ | Settings | `/settings` | Server, keybindings, updates, rune alerts, logging |

**Sidebar footer:**
- App version display
- Update badge (dot indicator when update available)

**Active state:**
- 3px gold accent bar on left edge
- `bg-elevated` background on active item

### Activity Ticker

Collapsible strip at the bottom of the window. Shows the last 1-2 actions in a single scrolling line (terminal style). Click to expand into full Activity Log page.

---

## 3. Pages

### 3.1 Dashboard (Home)

Route: `/`

The landing page providing high-level overview with quick access to key controls.

**Sections (vertical stack):**

**Quick Controls:**
- "GSI Automation" — pill toggle switch (ON/OFF)
- "Standalone Script" — pill toggle switch with combo key label

**Active Hero Card:**
- When hero detected: hero name + icon, summary of active keybindings, link to hero config page
- When no hero: compact grid of hero icons for manual override selection (8 heroes + "None")
- Auto-updates when GSI detects a new hero

**Recent Activity:**
- Last 5 entries from the activity feed
- Timestamps + color-coded entries (green=action, red=danger, yellow=warning, cyan=system)
- "View Full Log →" link to Activity page

### 3.2 Heroes

Route: `/heroes` (selector grid), `/heroes/:name` (individual hero page)

**Hero Selector Grid:**
- 2×4 grid of hero cards (icon + abbreviated name)
- Active hero highlighted with gold accent border
- Click any hero to open its dedicated config page

**Supported heroes:**

| Display Name | Internal Name | Icon |
|-------------|---------------|------|
| Broodmother | `npc_dota_hero_broodmother` | 🕷️ |
| Huskar | `npc_dota_hero_huskar` | 🔥 |
| Largo | `npc_dota_hero_largo` | 🎵 |
| Legion Commander | `npc_dota_hero_legion_commander` | ⚔️ |
| Meepo | `npc_dota_hero_meepo` | 🐾 |
| Outworld Destroyer | `npc_dota_hero_obsidian_destroyer` | 🌀 |
| Shadow Fiend | `npc_dota_hero_nevermore` | 👻 |
| Tiny | `npc_dota_hero_tiny` | 🪨 |

**Individual Hero Page Structure:**

Every hero page follows a consistent layout with collapsible card sections. Heroes with simpler configs have fewer sections.

1. **Header:** "← Back to Heroes" + hero name + icon
2. **Keybindings section:** Standalone combo key, ability keys (hero-specific)
3. **Hero-Specific Config:** Varies per hero (see details below)
4. **Danger Abilities:** Hero-specific danger responses (if applicable)
5. **Armlet Override:** Optional per-hero armlet settings
6. **Live Status:** Real-time observed state (only visible when hero is in-game)

All values are inline-editable. Changes auto-save with debounced writes to `config/config.toml`.

#### 3.2.1 Broodmother Page

**Keybindings:**
- Standalone Key (not actively used)

**Spider Micro:**
- Enable Spider Micro: toggle
- Spider Control Group Key: key input (default: F3)
- Reselect Hero Key: key input (default: 1)

**Auto-Items on Space+Right-Click:**
- Enable Auto Items: toggle
- Auto Items Modifier: display "Space" (hardwired)
- Item list: tag/chip list with × to remove, + to add. Default: orchid, bloodthorn, diffusal_blade, disperser, nullifier, abyssal_blade
- Auto Abilities First: toggle (default OFF)

**Auto Abilities:**
- Table of abilities: index, key, HP threshold (optional)
- Default: Insatiable Hunger (Q, HP < 80%), Spawn Spiderlings (R, always)

**Live Status:**
- Spider micro state, auto-items state

#### 3.2.2 Huskar Page

**Keybindings:**
- Standalone Key: key input (default: Home) — not actively used

**Berserker Blood:**
- Berserker Blood Key: key input (default: E)
- Cleanse Delay: number input (default: 300ms)

**Armlet Override:**
- Override toggle
- Threshold: 120 HP, Offset: 150 HP, Cooldown: 300ms

**Live Status:**
- Debuff state, Berserker Blood cooldown

#### 3.2.3 Largo Page

**Keybindings:**
- Standalone Key: display (not actively read)
- Q/W/E/R Ability Keys: key inputs

**Amphibian Rhapsody:**
- Enable: toggle
- Beat Interval: number input (default: 995ms)
- Beat Correction: number input (default: 30ms)
- Correction Every N Beats: number input (default: 5)

**Auto Behavior:**
- Auto Toggle on Danger: toggle
- Mana Threshold: slider (default: 20%)
- Heal HP Threshold: slider (default: 50%)

**Live Status:**
- Current song, beat counter, ultimate mode

#### 3.2.4 Legion Commander Page

**Keybindings:**
- Standalone Combo Key: key input (default: Home)

**Combo Sequence:**
- Display read-only combo order: W (Press The Attack) → Blade Mail → Mjollnir → BKB → Blink → Orchid/Bloodthorn → Duel (R) → Overwhelming Odds (Q)
- Soul Ring integration note

**Armlet Override:**
- Override toggle + threshold/offset/cooldown inputs

**Live Status:**
- Combo readiness, item availability

#### 3.2.5 Meepo Page

**Keybindings:**
- Standalone Combo Key: key input (default: Home)
- Earthbind Key: key input (default: Q)
- Poof Key: key input (default: W)
- Dig Key: key input (default: E)
- MegaMeepo Key: key input (default: R)

**Combo Settings:**
- Post-Blink Delay: number input (default: 80ms)
- Combo Items: tag/chip list (default: sheepstick, disperser) with + to add
- Combo Item Spam Count: number (default varies)
- Combo Item Delay: number (default varies)
- Earthbind Presses: number (default: 2), Interval: number
- Poof Presses: number (default: 3), Interval: number

**Danger Abilities:**
- Auto-Dig on Danger: toggle + HP threshold slider (default: 32%)
- Auto-MegaMeepo on Danger: toggle + HP threshold slider (default: 45%)
- Defensive Trigger Cooldown: number input (default: 1500ms)

**Farm Assist:**
- Toggle Key: key input (default: End)
- Pulse Interval: number input (default: 700ms)
- Min Mana: slider (default: 35%)
- Min Health: slider (default: 45%)
- Right-Click After Poof: toggle
- Suspend on Danger: toggle
- Suspend After Manual Combo: number input (default: 2500ms)

**Armlet Override:**
- Override toggle + threshold/offset/cooldown inputs

**Live Status (in-game only):**
- Ability readiness: Poof, Dig, MegaMeepo (✅ Ready / ❌ Cooldown)
- Aghanim's Shard / Scepter presence
- Blink slot key
- Castable combo item keys
- Farm assist mode: Inactive / Armed / Suspended (with reason)
- Farm pulse counter

#### 3.2.6 Outworld Destroyer Page

**Keybindings:**
- Standalone Combo Key: key input (default: Home)
- Objurgation Key: key input
- Arcane Orb Key: key input
- Astral Imprisonment Key: key input

**Auto-Objurgation on Danger:**
- Enable: toggle
- HP Threshold: slider (default: 55%)
- Min Mana: slider (default: 25%)
- Trigger Cooldown: number input (default: 1500ms)

**Ultimate Intercept:**
- Enable: toggle
- Auto-BKB on Ultimate: toggle
- Auto-Objurgation on Ultimate: toggle
- Post-BKB Delay: number input
- Post-Blink Delay: number input

**Standalone Combo:**
- Combo Items: tag/chip list (default: sheepstick, bloodthorn)
- Combo Item Spam Count: number
- Combo Item Delay: number
- Post-Ultimate Arcane Orb Presses: number
- Arcane Orb Press Interval: number

**Self-Astral Panic:**
- Enable: toggle
- Panic Key: key input (default: F5)

**Live Status:**
- Objurgation cooldown, ultimate readiness, combo state

#### 3.2.7 Shadow Fiend Page

**Keybindings:**
- Standalone Key: display (not actively read)

**Raze Intercept:**
- Enable: toggle
- Raze Delay: number input (default: 10ms)
- Note: "Intercepts Q/W/E to face cursor direction before razing"

**Ultimate Intercept:**
- Auto-BKB on Ultimate: toggle
- Auto-D Ability on Ultimate: toggle

**Live Status:**
- Raze intercept state, ultimate readiness

#### 3.2.8 Tiny Page

**Keybindings:**
- Standalone Combo Key: key input (default: Home)

**Combo Sequence:**
- Display read-only combo order: Blink → Avalanche (W, with Soul Ring) → W×3 → Toss (Q)×4 → Tree Grab (D)×3
- Soul Ring integration note

**Live Status:**
- Combo readiness

### 3.3 Danger Detection

Route: `/danger`

**Core Settings:**
- Enable Danger Detection: toggle
- HP Threshold: slider with label (range: 30–90%, default: 70%)
- Rapid Loss Threshold: slider (range: 50–300 HP, default: 100)
- Burst Time Window: slider (range: 200–1000ms, default: 500ms)
- Clear Delay: slider (range: 1–10s, default: 3s)

**Healing in Danger:**
- Healing HP Threshold: slider (range: 30–80%, default: 50%)
- Max Healing Items Per Event: slider (range: 1–5, default: 3)
- Display healing priority order (read-only): Cheese → Greater Faerie Fire → Enchanted Mango → Magic Wand → Faerie Fire

**Defensive Items:**
2-column grid of toggle cards, each with:
- Item icon/name
- Toggle switch
- Brief effect description
- Sub-settings where applicable (Satanic HP threshold slider)

Items:
| Item | Default | Sub-setting |
|------|---------|-------------|
| Black King Bar | ON | — |
| Satanic | ON | HP threshold slider (default: 40%) |
| Blade Mail | ON | — |
| Glimmer Cape | ON | Self-cast (double-tap) |
| Ghost Scepter | ON | — |
| Shiva's Guard | ON | — |

**Dispels:**
- Auto-Manta on Silence: toggle (default: ON)
- Auto-Lotus on Silence: toggle (default: ON)

**Neutral Items:**
- Enable Neutral Items: toggle
- Use in Danger Only: toggle
- HP Threshold: slider (default: 50%)
- Self-Cast Key: key input (default: 0)
- Allowed Items: tag/chip list with × to remove and + to add
  - Default list: essence_ring, minotaur_horn, jidi_pollen_bag, metamorphic_mandible, riftshadow_prism, kobold_cup, polliwog_charm, ash_legion_shield, mana_draught, pogo_stick, psychic_headband, crippling_crossbow, idol_of_screeauk, flayers_bota

### 3.4 Soul Ring

Route: `/soul-ring`

**Settings:**
- Enable Soul Ring: toggle
- Min Mana to Trigger: slider (range: 0–100%, default: 100%)
- Min Health Safety Floor: slider (range: 0–50%, default: 20%)
- Delay Before Ability: number input (default: 30ms)
- Trigger Cooldown: number input (default: 10ms)

**Intercepted Keys:**
- Ability Keys: multi-key input showing Q, W, E, R, D, F (configurable)
- Intercept Item Keys: toggle (default: ON)
- Info note: "Soul Ring pre-casts before these keys when mana is below threshold. Excludes Blink, TP, BKB, Armlet, and consumables."

### 3.5 Armlet

Route: `/armlet`

**Shared Settings:**
- Enable Armlet: toggle
- Cast Modifier: dropdown (default: Alt)
- Toggle Threshold: number input (default: 320 HP)
- Predictive Offset: number input (default: 30 HP)
- Toggle Cooldown: number input (default: 250ms)

**Per-Hero Overrides:**
- Summary list of heroes with custom armlet settings
- Each links to the respective hero page's Armlet Override section
- Shows current override values inline

### 3.6 Activity Log

Route: `/activity`

**Header controls:**
- Pause / Resume button (freezes auto-scroll)
- Clear button (clears log)

**Category filters:**
Toggle chips: All | Actions | Danger | Errors | System

**Log display:**
- Terminal-style monospace font (`JetBrains Mono 12px`)
- Dark background (`terminal-bg: #0A0E14`)
- Green/amber text (`terminal-green: #00FF88`)
- Timestamps with millisecond precision
- Color-coded by category:
  - Green: actions (Soul Ring, BKB, armlet toggle, combo fired)
  - Red: danger events (danger detected, HP critical)
  - Amber: warnings
  - Cyan: system events (app start, GSI connect, hero detected)
- Auto-scroll to latest (bottom), pauses when user scrolls up
- Entries expandable on click for details (e.g., HP delta for danger events)

**Log entry format:**
```
> HH:MM:SS.mmm  [CATEGORY]  Description
```

### 3.7 Diagnostics

Route: `/diagnostics`

**GSI Pipeline:**
- Events Processed: counter
- Events Dropped: counter
- Queue Depth: current / max
- Connection Status: green/red indicator

**Synthetic Input:**
- Queue Depth: current
- Total Queued: counter
- Peak Depth: counter
- Completions: counter
- Drops: counter

**Keyboard Hook:**
- Status: Active / Inactive
- Currently Blocked Keys: list
- Soul Ring State: Ready / Triggered / Cooldown

### 3.8 Settings

Route: `/settings`

**Server:**
- GSI Port: number input (default: 3000)
- Info note: "Restart required after changing port"

**Keybindings:**
- Item Slots 1–6: key inputs (defaults: Z, X, C, V, B, N)
- Neutral Slot: key input (default: 0)
- Combo Trigger: key input (default: Home)

**Common:**
- Survivability HP Threshold: number input (default: 30%)

**Rune Alerts:**
- Enable: toggle
- Alert Lead Time: number input (default: 10s)
- Check Interval: number input (default: 120s)
- Audio Alert: toggle

**Logging:**
- Log Level: dropdown (debug, info, warn, error; default: info)

**Updates:**
- Check on Startup: toggle (default: ON)
- Include Pre-releases: toggle (default: OFF)
- Current Version: display
- "Check for Updates Now" button
- Status display (Up to date / Available / Error)

**Advanced (collapsed by default):**
- Minimap Capture (Experimental):
  - Enable: toggle
  - Minimap coordinates: x, y, width, height inputs
  - Capture Interval: number input
  - Sample Every N: number input
  - Artifact Output Dir: text input

---

## 4. Visual Design System

### 4.1 Color Palette

| Token | Hex | Usage |
|-------|-----|-------|
| `bg-primary` | `#0D0F12` | Main background — near-black with slight blue |
| `bg-secondary` | `#161A21` | Card/panel backgrounds |
| `bg-elevated` | `#1E2330` | Hover states, active sidebar item |
| `bg-input` | `#12151B` | Input fields, sliders |
| `border-default` | `#2A3040` | Card borders, dividers |
| `border-accent` | `#C8AA6E` | Active states — Dota gold |
| `text-primary` | `#E8E6E3` | Main text — warm white |
| `text-secondary` | `#8B9BB4` | Labels, descriptions — muted blue-grey |
| `text-muted` | `#4A5568` | Disabled, placeholder text |
| `accent-gold` | `#C8AA6E` | Primary accent — Dota gold (active tabs, toggles, links) |
| `accent-red` | `#E74C3C` | Danger, HP critical, errors |
| `accent-green` | `#2ECC71` | HP healthy, success, ON toggles |
| `accent-blue` | `#3498DB` | Mana bar, info badges |
| `accent-amber` | `#F39C12` | Warnings, rune timer flash |
| `terminal-green` | `#00FF88` | Activity log text (terminal style) |
| `terminal-bg` | `#0A0E14` | Activity log background |

### 4.2 Typography

| Element | Font | Size | Weight |
|---------|------|------|--------|
| Page headings | `Inter` | 20px | 600 (semibold) |
| Section headings | `Inter` | 16px | 600 (semibold) |
| Body text | `Inter` | 14px | 400 (regular) |
| Labels | `Inter` | 12px | 500 (medium) |
| Input values | `JetBrains Mono` | 13px | 400 |
| Activity log entries | `JetBrains Mono` | 12px | 400 |
| Status bar values | `JetBrains Mono` | 12px | 500 |

`Inter` provides clean readability for UI elements. `JetBrains Mono` gives the terminal/hacker aesthetic for data display, log output, and numeric inputs.

### 4.3 Component Specifications

**Cards/Panels:**
- Background: `bg-secondary`
- Border: 1px `border-default`
- Border radius: 8px
- Padding: 16px
- Collapsible via section header click

**Toggle Switches:**
- Pill-shaped, 36×20px
- ON: `accent-gold` fill, white knob
- OFF: `bg-input` fill, `text-muted` knob

**Sliders:**
- Track: `bg-input`, 4px height, rounded
- Filled portion: `accent-gold`
- Thumb: `text-primary`, 16px circle
- Live value label displayed to the right

**Input Fields:**
- Background: `bg-input`
- Border: 1px `border-default`, focus: 1px `border-accent`
- Border radius: 6px
- Font: `JetBrains Mono` 13px
- Height: 32px

**Buttons:**
- Primary: `accent-gold` fill, `bg-primary` text, 6px radius, 32px height
- Secondary: `bg-elevated` fill, `text-primary` text, 1px `border-default`
- Hover: lighten by 10%

**Tag/Chip Components (for item lists):**
- Background: `bg-elevated`
- Border: 1px `border-default`
- Border radius: 12px (pill)
- Padding: 4px 10px
- × button for removal
- "+ Add" chip with dashed border

**HP Bar:**
- Height: 8px (status bar), 16px (dashboard)
- Border radius: 4px
- Gradient: `accent-green` (100–60%) → `accent-amber` (60–30%) → `accent-red` (30–0%)
- Numerical value overlaid in `text-primary`

**Mana Bar:**
- Same dimensions as HP bar
- Solid `accent-blue`
- Numerical value overlaid

**Danger Badge:**
- Background: `accent-red`
- Text: white
- Border radius: 4px
- Subtle CSS pulse animation (opacity 0.7→1.0, 1.5s loop)

**Sidebar:**
- Width: 60px collapsed, 200px expanded
- Background: `bg-primary`
- Active item: 3px left border `accent-gold` + `bg-elevated` background
- Icons: 20px, `text-secondary` default, `accent-gold` when active
- Hover: `bg-elevated` background, smooth transition

### 4.4 Spacing System

- Base unit: 8px
- Component gap: 8px (within form groups)
- Section gap: 24px (between card sections)
- Card padding: 16px
- Page margin: 24px
- Sidebar item padding: 12px vertical, 16px horizontal

### 4.5 Animations

- Toggle switches: 200ms ease-in-out
- Sidebar expand/collapse: 200ms ease
- Danger pulse: opacity 0.7→1.0, 1.5s infinite
- Status bar transitions: 300ms for HP/mana bar width changes
- Page transitions: 150ms fade
- Hover effects: 150ms

---

## 5. Data Flow & IPC

### 5.1 Tauri Command Bridge

The React frontend communicates with the Rust backend via Tauri IPC commands.

**Read commands (Rust → React):**
- `get_config()` → returns full config as JSON
- `get_app_state()` → returns current AppState snapshot (hero, enabled flags, metrics)
- `get_game_state()` → returns latest GSI event data (hero stats, items, abilities)
- `get_danger_state()` → returns current danger detection state
- `get_activity_log()` → returns recent activity entries
- `get_diagnostics()` → returns GSI queue metrics, synthetic input metrics, keyboard hook state
- `get_update_state()` → returns current UpdateCheckState

**Write commands (React → Rust):**
- `update_config(section, key, value)` → updates config and writes to config.toml
- `set_gsi_enabled(enabled)` → toggles GSI automation
- `set_standalone_enabled(enabled)` → toggles standalone script
- `select_hero(hero)` → manual hero override
- `check_for_updates()` → triggers update check
- `apply_update()` → downloads and applies update
- `dismiss_update()` → dismisses update banner

**Event streams (Rust → React, push-based):**
- `gsi_update` → emitted on each GSI event (throttled to ~10/sec for UI)
- `danger_state_changed` → emitted when danger state transitions
- `activity_event` → emitted when an automation action fires
- `update_state_changed` → emitted on update state transitions
- `hero_changed` → emitted when active hero changes

### 5.2 Config Persistence

- All config changes write to `config/config.toml` via Tauri commands
- Changes are debounced (300ms) on the frontend before sending
- Backend validates values before writing
- Some changes (like server port) flag "restart required"

---

## 6. State Management

### 6.1 Frontend State

Use a lightweight state manager (Zustand recommended for simplicity):

**Stores:**
- `configStore` — mirrors config.toml, synced via Tauri commands
- `gameStore` — real-time game state from GSI events
- `uiStore` — UI-only state (sidebar collapsed, active page, filter selections)
- `activityStore` — ring buffer of recent activity events
- `updateStore` — update check state

### 6.2 Reactive Updates

- GSI events pushed from Rust via Tauri event system
- React components subscribe to relevant store slices
- Status header bar re-renders on `gameStore` changes
- Activity log appends on `activity_event` events
- Config changes trigger debounced write-back

---

## 7. Responsive Behavior

### Window Sizes

| Width | Sidebar | Content | Activity Ticker |
|-------|---------|---------|-----------------|
| < 900px | Not supported (min width) | — | — |
| 900–1100px | Collapsed (60px, icons only) | Full width minus sidebar | Collapsed to single line |
| > 1100px | Expanded (200px, icons + labels) | Full width minus sidebar | Expanded with 2-3 entries |

### Sidebar Interaction

- At collapsed width: hover over sidebar temporarily expands it (200ms delay, overlays content)
- Click to navigate, sidebar re-collapses after navigation at narrow widths

---

## 8. Error Handling

### Config Validation

- Invalid values show inline red border + error message below field
- Invalid values are not written to config.toml
- Range constraints enforced by sliders (can't go out of bounds)
- Key inputs validated for conflicts (e.g., same key assigned to two functions → warning)

### Connection States

- GSI disconnected: status bar shows red dot + "Disconnected"
- Keyboard hook failure: diagnostics page shows error state
- Config file write failure: toast notification with error message

### Graceful Degradation

- If Tauri IPC fails, show "Connection to backend lost" banner
- Activity log continues to accumulate locally even if backend temporarily unavailable
- All toggles and inputs disabled when backend connection lost

---

## 9. Accessibility

- All interactive elements keyboard-navigable (Tab order)
- Focus indicators using `border-accent` (gold) ring
- Toggle states announced via aria-labels
- Minimum 4.5:1 contrast ratio for text on backgrounds
- Slider values readable by screen readers via aria-valuetext

---

## 10. Technology Stack

| Layer | Technology |
|-------|-----------|
| Desktop shell | Tauri v2 |
| Frontend framework | React 18+ with TypeScript |
| Build tool | Vite |
| State management | Zustand |
| Styling | Tailwind CSS v4 with custom design tokens from Section 4 |
| Fonts | Inter (Google Fonts), JetBrains Mono (Google Fonts) |
| Icons | Lucide React (MIT, consistent style) |
| Routing | React Router v6 |
| IPC | @tauri-apps/api |

---

## 11. File Structure (Frontend)

```
src-ui/
├── public/
│   └── fonts/
├── src/
│   ├── App.tsx
│   ├── main.tsx
│   ├── styles/
│   │   ├── tokens.css          # Design system CSS custom properties
│   │   ├── global.css          # Reset, base styles
│   │   └── components/         # Component-specific styles
│   ├── components/
│   │   ├── layout/
│   │   │   ├── Sidebar.tsx
│   │   │   ├── StatusHeader.tsx
│   │   │   └── ActivityTicker.tsx
│   │   ├── common/
│   │   │   ├── Toggle.tsx
│   │   │   ├── Slider.tsx
│   │   │   ├── KeyInput.tsx
│   │   │   ├── NumberInput.tsx
│   │   │   ├── TagList.tsx
│   │   │   ├── Card.tsx
│   │   │   ├── Button.tsx
│   │   │   ├── HPBar.tsx
│   │   │   └── ManaBar.tsx
│   │   └── heroes/
│   │       ├── HeroGrid.tsx
│   │       ├── HeroPage.tsx    # Shared hero page shell
│   │       └── configs/        # Per-hero config sections
│   │           ├── BroodmotherConfig.tsx
│   │           ├── HuskarConfig.tsx
│   │           ├── LargoConfig.tsx
│   │           ├── LegionCommanderConfig.tsx
│   │           ├── MeepoConfig.tsx
│   │           ├── OutworldDestroyerConfig.tsx
│   │           ├── ShadowFiendConfig.tsx
│   │           └── TinyConfig.tsx
│   ├── pages/
│   │   ├── Dashboard.tsx
│   │   ├── Heroes.tsx
│   │   ├── DangerDetection.tsx
│   │   ├── SoulRing.tsx
│   │   ├── Armlet.tsx
│   │   ├── ActivityLog.tsx
│   │   ├── Diagnostics.tsx
│   │   └── Settings.tsx
│   ├── stores/
│   │   ├── configStore.ts
│   │   ├── gameStore.ts
│   │   ├── uiStore.ts
│   │   ├── activityStore.ts
│   │   └── updateStore.ts
│   ├── hooks/
│   │   ├── useTauriCommand.ts
│   │   ├── useTauriEvent.ts
│   │   └── useDebounce.ts
│   └── types/
│       ├── config.ts
│       ├── game.ts
│       └── activity.ts
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
└── tailwind.config.ts
```

---

## 12. Out of Scope

The following are explicitly out of scope for this design:

- Mobile/tablet responsive layouts (desktop-only Tauri app)
- Multi-language / i18n support
- User accounts or cloud sync
- In-game overlay (HUD overlay rendered on top of Dota 2)
- New automation features beyond what currently exists
- Rust backend refactoring (only adding Tauri command wrappers)
- Custom themes / theme switching (single dark theme)
