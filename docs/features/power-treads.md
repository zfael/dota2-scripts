# Power Treads Switching

## Purpose

How the app switches Power Treads for you: INT before a cast that costs mana,
STR while you are in danger, the smallest pool while a heal runs, and back
home otherwise. It also covers how the app knows which attribute the treads
are on, when GSI never says.
**Read this when:** turning the feature on, tuning it, or debugging a switch
that went wrong.

## Feature Summary

- **Off by default.** `enabled = false` until it has proven itself in real games.
- **Cast switch.** Before a spell or item that costs mana (priced by the same
  table Soul Ring uses), switch to the attribute with the most mana. Chained
  casts stay there, and the treads go home once the mana has been spent.
- **Danger switch.** While the danger detector is firing, hold STR. This beats
  every other behaviour, so casts in danger do not leave STR.
- **Heal switch.** Bottle, Salve, Tango, Clarity, Wand and similar are used in
  the smallest pool. A switch keeps your HP% and mana%, so a flat heal taken
  in a small pool is worth more once you switch back.
- **Verify.** When you are idle and safe, the app cycles the treads once to find
  or confirm the state.
- **Your switches count.** A switch you make yourself while idle, by key or
  by mouse click, becomes the new home.

Every behaviour has its own toggle, on the **Boots** page or in `[power_treads]`.

## Configuration

```toml
[power_treads]
enabled = false
cast_switch = true
danger_switch = true
heal_switch = true
auto_verify = true
manual_switch_sets_home = true
min_health_percent = 30
confirm_timeout_ms = 700
cast_linger_ms = 800
cast_timeout_ms = 3000
verify_interval_ms = 120000
verify_quiet_ms = 3000
verify_retry_ms = 5000
heal_grace_ms = 700
heal_settle_ms = 700
heal_max_ms = 15000
press_gap_ms = 30
```

| Option | Default | UI | Description |
|---|---|---|---|
| `enabled` | `false` | Boots | Master switch. The state is tracked even while off. |
| `cast_switch` | `true` | Boots | Switch to the most-mana attribute before a mana-costing spell or item |
| `danger_switch` | `true` | Boots | Hold STR while the danger detector fires |
| `heal_switch` | `true` | Boots | Use heals in the smallest pool; never while in danger |
| `auto_verify` | `true` | Boots | Cycle the treads when idle to find or confirm the state |
| `manual_switch_sets_home` | `true` | Boots | Your own idle switch becomes home |
| `min_health_percent` | `30` | Boots | Below this HP%, never leave a higher-HP attribute for a cast |
| `confirm_timeout_ms` | `700` | config only | A press that shows nothing after this long was dropped |
| `cast_linger_ms` | `800` | config only | Stay on the cast attribute this long after the mana is spent |
| `cast_timeout_ms` | `3000` | config only | Stop waiting for the spend (cancelled cast, walking to range) |
| `verify_interval_ms` | `120000` | config only | Re-verify a confident state this often; `0` = never |
| `verify_quiet_ms` | `3000` | config only | Verify only after this long without an ability, item or treads key |
| `verify_retry_ms` | `5000` | config only | Back-off after a failed verify |
| `heal_grace_ms` | `700` | config only | Time for a heal to show up before "not rising" can end it |
| `heal_settle_ms` | `700` | config only | End the heal once its pool has stopped rising this long |
| `heal_max_ms` | `15000` | config only | Longest a heal can hold the treads away from home |
| `press_gap_ms` | `30` | config only | Gap between treads presses, and before the original key |

The timings are in milliseconds and stay in `config.toml`. The UI exposes the
switches and the HP gate, which are the parts you would change mid-session.

## Related Files

| File | Purpose |
|---|---|
| `src/actions/power_treads.rs` | The state machine, GSI observation, and key planning |
| `src/actions/hero_attributes.rs` | **Generated** hero → primary attribute table |
| `scripts/generate-hero-attributes.ps1` | Regenerates it from odota/dotaconstants |
| `src/input/keyboard.rs` | Sends the planned treads presses ahead of the key, before Soul Ring |
| `src/actions/dispatcher.rs` | Feeds every GSI event to `power_treads::observe_event` |
| `src/config/settings.rs` | `PowerTreadsConfig` |
| `examples/power_treads_probe.rs` | Stage 1 probe: what a switch looks like in GSI. The re-measure tool after a patch |
| `examples/power_treads_control.rs` | Stage 2 simulator: runs the real machine against a simulated game |

---

## Details

### What the game told us

GSI lists `item_power_treads` but has no field for its attribute. The item
object is byte-identical in all three states. The state has to be inferred.
Measured with `examples/power_treads_probe.rs` on 2026-09-21, on Earth Spirit,
Slark, Ogre Magi, Void Spirit and Outworld Destroyer:

| Finding | Value |
|---|---|
| Treads attribute in GSI | none |
| Start state after purchase | STR hero → STR, AGI → AGI, INT → INT, universal → STR |
| One switch, ordinary hero | 220 max HP / 120 max mana |
| HP% / mana% across a switch | preserved, 46/46 |
| Press → change visible in GSI | 13–251ms; GSI tick ~228ms median |
| Two presses inside one tick | read as one two-step switch |
| Ogre Magi | STR also gives mana, INT gives none: INT and AGI look identical |
| Outworld Destroyer | Astral Imprisonment: +1250 max mana with no press |
| Press during a cast point or channel | does not cancel it |
| Press while stunned | does nothing |

### Reading the state

The treads cycle **STR → INT → AGI → STR**, one step per press. For an
ordinary hero, each switch moves the two pools differently:

| Switch | presses | max HP | max mana |
|---|---|---|---|
| STR → INT | 1 | − | + |
| INT → AGI | 1 | 0 | − |
| AGI → STR | 1 | + | 0 |
| STR → AGI | 2 | − | 0 |
| INT → STR | 2 | + | − |
| AGI → INT | 2 | 0 | + |

That sign table is only the **seed**. Two captures broke it:

- **Outworld Destroyer's** Astral Imprisonment adds 1250 max mana with the same
  sign as AGI → INT.
- **Ogre Magi** gets mana from STR and none from INT.

So the module learns each attribute's exact `(max HP, max mana)` offset
relative to STR, per hero. A change counts as a switch only when it matches a
learned switch **exactly**. Anything else marks the state *suspect*, and a
verify settles it. A match by sign alone, before the offsets are learned, is
also only suspect.

### Confidence

| State | Meaning | What runs |
|---|---|---|
| Confident | Estimate matches every change seen | Everything |
| Suspect | Probably right, but something did not add up | No switching; verify at the next safe moment |
| Unknown | No idea — app started with treads owned, or a verify failed | No switching; verify at the next safe moment |

These make the state suspect:

- a press that showed nothing (dropped while stunned, say);
- a change no learned switch explains (a buff);
- a switch landing on a level-up or item-change tick;
- the treads leaving the main inventory;
- death with presses in flight.

### Verify

When idle, safe, and above `min_health_percent`, the app presses the treads
three times, one press per GSI tick, and ends where it started. The attribute
with the most max HP is STR, which is true on every hero, and the fixed
STR → INT → AGI order names the other two. It works on Ogre Magi too, whose
INT → AGI step changes nothing. That step is recorded as a zero step after
`confirm_timeout_ms`. A cycle that does not add back up to zero, because a
press was dropped, fails and retries after `verify_retry_ms`.

A verify also runs:

- once after buying treads, after `verify_quiet_ms`, to learn the exact
  offsets;
- every `verify_interval_ms` while confident.

Any ability, item or treads key aborts it.

**One trade-off:** until that first verify has learned the offsets, a cast's
switch is matched by sign only, so it leaves the state suspect. The treads
then wait on the cast attribute until the verify runs, about 3–4 seconds,
before going home. It happens once per purchase.

### Targets

Targets come from the learned offsets, not from attribute names:

| Behaviour | Target | Ordinary hero | Ogre Magi |
|---|---|---|---|
| Cast | Most max mana | INT | STR (no switch) |
| HP heal | Least max HP, then fewest presses | INT or AGI | INT or AGI |
| Mana heal | Least max mana | AGI | INT or AGI |
| Bottle, Wand (both) | Least of both | AGI | INT or AGI |
| Danger | STR | STR | STR |

A cast is detected as done when the mana fraction drops. A switch keeps the
fraction, so this works across a switch too. The cast target is kept for
`cast_linger_ms` after that, or dropped after `cast_timeout_ms` if no spend is
seen.

A heal ends when its pool stops rising faster than your usual regen, when you
take damage (Salve breaks), when danger starts, or when the pool is full.

### Keyboard ordering

The treads presses are planned in the generic ability/item branch of the
keyboard hook, after every hero intercept that blocks the key. A switch is
therefore never planned for a press that will not go out. They go out ahead
of Soul Ring and the original key, on the same replay worker:

```
key pressed → treads × n → Soul Ring (if due) → original key
```

These keys get no switch:

- keys a hero script blocks and re-presses itself (SF razes, OD's R, Largo's
  songs, and so on);
- Ember Spirit's Sleight of Fist, whose Fire Remnant activate is timed from
  the key press;
- item slots bound to a mouse button (the hook plans on key presses only).

A mouse click on the treads icon is not seen as a key press, but the switch
still shows up in GSI and is followed as an unannounced switch.

### Hero attributes

`src/actions/hero_attributes.rs` is generated from `heroes.json` in
odota/dotaconstants:

```powershell
pwsh scripts/generate-hero-attributes.ps1
```

Rerun it when a patch adds a hero or changes a hero's primary attribute.
Universal heroes, and any hero missing from the table, start on STR.

### After a gameplay patch

Re-run the probe (`cargo run --example power_treads_probe -- --port 3000` with
the app closed) and follow the capture protocol in its header. Nothing in the
logic depends on the 220 / 120 sizes, so a change there needs no code change.
A change to the cycle order, or to what STR grants, would.

### Logging

With `level = "debug"`, every decision is logged with a 👟 prefix:

```
👟 SpendMana: AGI → INT (2 press)
👟 suspect: matched before the switch sizes were learned
👟 verified INT — offsets [Some((0, 0)), Some((-220, 120)), Some((-220, 0))]
👟 move INT → AGI (1 press)
```

At `info`, only the presses the GSI side sends are logged
(`👟 Power Treads: N press(es)`).
