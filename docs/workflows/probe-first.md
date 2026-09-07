# Probe First

**Purpose**: Use this page before writing automation whose trigger depends on
game state GSI does not report directly, or that presses keys in a fight.

The short version: **measure the game before you model it, and prove the control
loop before it can press anything.** The evidence lives in `examples/`, and it
stays there so it can be re-run after a patch.

---

## Does this apply to what you are building?

| Question | If yes |
|---|---|
| Does the trigger depend on something GSI has no field for — a toggle's state, whether a channel is running, a modifier, whether a cast actually landed? | Stage 1: probe |
| Will it press keys during a fight, where a wrong press costs the game? | Stage 2: simulate |
| Neither — it reads a field that plainly means what it says, and acts out of combat | Skip both; the loop in `docs/workflows/testing-and-debugging.md` is enough |

GSI exposes no modifiers block at all, so the first case comes up constantly.
The repo is already full of inference: Slark reads a `0 -> N` cooldown edge as a
Shadow Dance cast, `defensive_windows.rs` reads the same edge for Glimmer and
Ghost Scepter, Morphling reads `max_health` deltas as an Attribute Shift. Each of
those is a guess about the game that has to be checked against the game.

---

## Stage 1 — Probe: find out what the game actually says

**Never assume a field means what its name suggests.** `abilities.abilityN.ability_active`
is the cautionary tale: it is the obvious way to read a toggle, it is in our
schema, and a live capture shows it is `true` for every ability on every tick —
including `plus_guild_banner` and `plus_high_five`. An implementation built on
the name would have looked correct in review and never worked in a game.

Write `examples/<hero>_<subject>_probe.rs` that:

- **listens on its own port**, so it runs next to the real app. Dota supports
  multiple `gamestate_integration` cfg files — a second one pointing at
  `127.0.0.1:3100` gives the probe the same stream without disturbing anything.
- **records raw payloads verbatim** under `logs/`, so a session can be
  re-analysed without playing again. Dota pretty-prints its JSON, so a payload
  spans many lines: read captures back as a **stream of JSON values**, not line
  by line.
- **prints a derived timeline live**, not just a dump — the point is to see the
  thing you are trying to detect as it happens.
- **has a `--replay <file>` mode** that produces the same analysis offline.
- **watches the relevant keys** with `rdev::listen` (observing, unlike the app's
  `rdev::grab`), when press-to-effect latency matters.

Put the **capture protocol in the file header** as a numbered table: what to do
in game, and what each step measures. Include the steps that should *not*
trigger your detector — level up, buy an item, swap Power Treads — so the
discriminator is tested, not just the happy path.

Then report **measured numbers in a table**. "It seems to work" is not a
finding; "226ms median tick, 95–391ms press latency, every step a multiple of 22
HP" is.

## Stage 2 — Simulate: prove the loop before it presses anything

Skip this if the automation only reads. If it acts, build
`examples/<hero>_<subject>_control.rs` holding three things:

1. **The state machine**, written in the shape it will take in `src/` so the
   promotion is a move rather than a rewrite.
2. **A simulator of the world**, wired with the constants Stage 1 measured —
   tick interval, rate, press latency, limits.
3. **A scenario suite** with assertions that print PASS/FAIL and exit non-zero
   on failure.

Scenarios must include the ugly ones, because those are the ones that bite:

- the trigger firing again while the action is still running
- the resource running out mid-action
- the player doing the same thing manually, unannounced
- the trigger returning while the automation is undoing its own work
- the feature disabled

Anything Stage 1 could **not** measure gets modelled both ways, and the suite has
to pass under both. Morphling's capture ran at full HP throughout, so it could
not say whether losing max HP drags current HP down or merely clamps it; the
suite runs under `--hp-model pessimistic` and `--hp-model clamped`.

## Stage 3 — Promote

Move the state machine into `src/`, add the config struct, and **write unit
tests anyway**. The simulator is not a substitute: it exercises the loop against
one world, while unit tests pin individual decisions and catch what a plausible
world hides (see the table below).

Keep both probes. They are the re-measure tool after a gameplay patch, and the
hero doc should say so.

---

## The rule that makes it survive patches

**Measured constants describe the world the logic was tested against. They must
not appear in the logic.**

- Config thresholds go in units GSI reports — HP, seconds — never attribute
  points, tick counts, or anything a patch note can change.
- Any "how far will this move before my press lands" estimate comes from the
  **last delta actually observed**, not a stored rate.
- Show the derived-but-friendlier number in the UI or the logs if it helps
  (Morphling's panel converts the HP target to approximate strength points), but
  never let the code depend on it.

Applied to Morphling, that means 22 HP/point and 345 HP/s appear in the module
doc, the hero doc and the simulator — and nowhere in the decision loop.

---

## What this has actually caught

From the Morphling build, in order:

| Stage | Caught | Cost if it had shipped |
|---|---|---|
| Probe | `ability_active` is meaningless | The obvious implementation, silently never firing |
| Simulator | A sustained fight re-triggered every settle window: 13 presses in 12s, ratcheting to a full shift | Losing all damage in every long fight |
| Simulator | The stop press landed late enough to overshoot *past* the player's own baseline | Quietly rebuilding the hero the player did not ask for |
| Unit tests | The plateau check timed stillness from the last movement — for a stat that sits still all game, that meant declaring the resource empty on the first tick after any press | The feature latching itself off permanently, in every game |

The last row is why Stage 3 says to write the unit tests anyway: the simulator
had the same bug and passed, because its worlds happened to start moving on the
exact tick the settle window expired.

---

## Checklist

- [ ] Probe exists under `examples/`, with a capture protocol in its header
- [ ] Capture taken in a real game; raw payloads saved under `logs/`
- [ ] Measured findings reported as a table, including what the field you
      expected to use actually does
- [ ] Simulator + scenario suite exists, if the automation acts
- [ ] Anything unmeasured is modelled both ways and passes both
- [ ] No measured constant appears in the decision logic
- [ ] Unit tests written during promotion, not just the scenario suite
- [ ] Probes kept, and referenced from the hero doc as the re-measure tool
- [ ] Module doc cites the capture the numbers came from

---

## Related docs

- `docs/workflows/adding-a-hero.md` — where this sits in the hero checklist
- `docs/workflows/testing-and-debugging.md` — the normal loop, once the
  uncertainty is gone
- `docs/reference/gsi-schema-and-usage.md` — what the payload is known to
  contain, and which fields have already been found untrustworthy
- `docs/heroes/morphling.md` — the worked example
