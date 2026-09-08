---
name: dota-bug-watch
description: Check ValveSoftware/Dota2-Gameplay for gameplay bugs reported since the last run, and triage which ones this repo's automation could act on. Use when the user asks to check the Dota bug tracker, look for new glitches, or see what players are reporting.
---

# Dota bug watch

Pull new issues from Valve's public gameplay tracker, drop the store/matchmaking
noise, and report the ones that touch a hero we script, an item we automate, or
the input layer we drive.

## Run it

```powershell
./scripts/dota-bug-watch.ps1
```

That reads the cursor at `.cache/dota-bug-watch/cursor.json`, reports everything
filed since, and advances the cursor. `.cache/` is gitignored, so the cursor is
local to this machine.

Useful flags:

| Flag | When |
|---|---|
| `-NoAdvance` | Re-reading the same window without consuming it. Pair with `-Since` when exploring. |
| `-Since 2026-08-20` | Ignore the cursor and look back to a fixed date. |
| `-Days 14` | First run only — how far back to seed when there is no cursor. |
| `-Requests` | Include feature requests and balance opinions, hidden by default. |
| `-All` | No filtering at all. Use when the default report looks suspiciously empty. |
| `-Json` | Machine-readable output. |

Needs `gh` authenticated (`gh auth status`). It is a public repo, but the API
rate limit for anonymous requests will not survive the pagination.

## Tiers the script emits

- **ACTIONABLE** — matched a hero in `src/actions/heroes/`, an item in
  `src/actions/`, or the input layer (smart cast, double tap, hotkeys, unit
  select). Hero hits are tagged with the hero name.
- **MECHANIC** — a real ability/item defect, but nothing we drive today. Worth
  a skim; a mechanic can become relevant when a hero is added.
- **REQUEST / NOISE / OTHER** — suppressed unless asked for.

## Then triage

The script ranks by keyword; it cannot tell a real defect from a confused
player. Read the ACTIONABLE list yourself and sort each into:

1. **Usable now** — a deterministic behaviour we can trigger or must avoid.
   Give the user the issue number, what it changes, and which file in
   `src/actions/` it lands in.
2. **Watch** — plausible but unproven, or no match ID. Note it, do not act.
3. **Discard** — working as intended, or a player misreading a tooltip.

Two things carry more weight than reaction count:

- **A reproducible demo-mode repro** beats a match ID, which beats a bare claim.
  Reports with neither ("### Example Match ID: 123") are close to worthless.
- **Input-layer bugs outrank hero bugs.** A bug in smart cast or hotkey
  handling changes what our own synthetic keystrokes do, in every hero.

Anything that survives triage is a candidate for automation, not a fact.
`docs/workflows/probe-first.md` applies: a forum report is a hypothesis about
game state, so measure it with a probe under `examples/` before writing an
action that depends on it. Valve also fixes these without warning — a behaviour
we lean on can vanish in any patch, so anything built on one needs a fallback
for when it stops being true.

## Reporting back

Lead with the ACTIONABLE items and what they mean for this repo. Summarise
MECHANIC in a line or two. Do not paste the raw script output — the user wants
the triage, not the feed.
