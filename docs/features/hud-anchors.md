# HUD Anchors

Calibrated screen positions for points on Dota's own HUD that automation clicks.

**Owner:** `src/observability/hud_anchors.rs`
**Config:** `[hud]` — see `docs/reference/configuration.md`
**Commands:** `src-tauri/src/commands/hud.rs`

**Status:** portrait anchor shipped. It is the only anchor so far; the section is
named for the category rather than the hero so a second one can be added without
moving anything.

---

## Why this exists

Some Dota abilities cannot be self-cast. Double-tapping the key does nothing and the
ALT modifier does nothing — the game aims them wherever the mouse happens to be. The
techniques that work elsewhere in this codebase (Snapfire's ALT-held cookie, Lotus Orb
and OD's Astral double-tap) do not apply, because those abilities support self-cast
in-game and these do not.

The one thing that does work is **clicking the hero portrait**: Dota resolves a click on
the portrait as a click on the hero, so a point-target ability lands underneath him.

That needs a screen coordinate, and there is no way to derive one. Where the portrait
sits inside Dota's window depends on resolution, UI scale, and HUD skin. So it is
measured once by the user and stored.

Current consumer: the Slark shard fallback (`docs/heroes/slark.md`).

---

## Coordinate basis

Anchors are stored as **fractions of Dota's client rect**, not of the display.

A fraction survives moving the window, changing resolution, and a second monitor; a
screen pixel survives none of those. Resolution back to pixels happens at use time
against the live window rect via `find_dota2_client_screen_rect()`, the same helper the
wave overlay uses for placement.

---

## Failing safe

`portrait_calibrated` defaults to **false**, and `resolve_portrait_point` returns `None`
until it is true. Callers must treat `None` as "do not click".

This matters more than it looks. A stray click in Dota is a move order — a mis-aimed
portrait click during a fight would walk your hero somewhere you did not ask for. The
shipped `portrait_x_fraction` / `portrait_y_fraction` are a starting point for the Test
button only; nothing acts on them until a real measurement replaces them.

`point_from_fraction` additionally clamps to `[0, 1]`, so even a hand-edited config
cannot produce a click outside Dota's window.

---

## Calibrating

Either path writes the same fields and broadcasts the config so open windows update
without a restart:

| Path | How |
|---|---|
| Hotkey | Hover the centre of the hero portrait in game, press `[hud] capture_portrait_key` (default `F9`) |
| UI | **Settings → HUD Anchors → Capture Now**, with Dota visible |

The capture reads the cursor position and Dota's client rect, and **rejects a cursor
outside Dota's window** rather than storing a fraction that points at the desktop.

**Test** parks the cursor on the stored anchor and leaves it there. It deliberately does
not click: it runs while Dota is focused, where a click would issue an order.

The capture hotkey is **blocked** from reaching Dota, like the wave overlay toggle, so
pick a key Dota does not need.

---

## Casting through an anchor

`input::simulation::portrait_cast(key, x, y)` runs one queued synthetic-input job:

1. Save the cursor position.
2. Press the ability key, putting it into targeting mode.
3. Move to the anchor, settle briefly so the hover registers, left-click.
4. Restore the cursor.

If the cursor move fails it **bails without clicking**, leaving the ability in targeting
mode rather than resolving it somewhere unintended.

The whole sequence sits inside the existing `SIMULATING_KEYS` guard, so the global hook
does not re-intercept our own synthetic input.

### The unavoidable cost

This moves the real mouse mid-fight. If the player is aiming or dragging at that exact
moment, their input and ours interleave, and there is no way around it — the ability
cannot be cast any other way. Consumers should expose a toggle so the behaviour can be
switched off without losing the rest of the automation.

---

## Tests

- `cargo test --lib hud_anchors` — fraction measured from the window origin rather than
  the screen, fraction↔point round-trip, cursor outside the window rejected, zero-sized
  window rejected, corrupt fraction clamped, uncalibrated anchor never resolves.
- `cargo test --lib keyboard` — the capture hotkey is parsed and plans its event.

**Not covered by automated tests** (they need a running Dota 2): whether the captured
point actually lands on the portrait, whether Dota resolves the click as a self-cast, and
cursor restoration against real input.
