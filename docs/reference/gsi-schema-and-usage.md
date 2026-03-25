# GSI Schema and Usage

**Purpose**: Use this page when adding or debugging behavior that starts from a Dota 2 GSI POST.

---

## Primary owners

| Path | What it owns |
|---|---|
| `src/models/gsi_event.rs` | Rust schema for the webhook body |
| `src/gsi/server.rs` | HTTP listener and bounded event queue |
| `src/gsi/handler.rs` | Request handler, optional JSONL logging, `AppState` updates |
| `src/actions/dispatcher.rs` | Pre-dispatch shared hooks and hero/common routing |
| `tests/gsi_handler_tests.rs` | Fixture-backed schema smoke tests |
| `tests/fixtures/` | Sample payloads you can copy when adding tests |

---

## Runtime flow

1. Dota 2 POSTs JSON to `http://127.0.0.1:<port>/`
2. `src/gsi/server.rs` binds the listener and creates a queue with capacity `10`
3. `src/gsi/handler.rs::gsi_webhook_handler()` deserializes `Json<GsiWebhookEvent>` and `try_send`s it
4. `src/gsi/handler.rs::process_gsi_events()`:
   - optionally writes JSONL when `[gsi_logging].enabled = true`
   - updates `AppState.last_event`
   - updates `AppState.metrics.current_queue_depth`
   - logs death/respawn transitions
   - checks `AppState.gsi_enabled`
   - spawns `ActionDispatcher::dispatch_gsi_event(...)`
5. `src/actions/dispatcher.rs` runs shared hooks before hero/common actions:
   - neutral-item discovery logging
   - Soul Ring state refresh
   - silence dispel checks
   - Broodmother active-hero flag updates
   - auto-items GSI cache refresh

**HTTP status behavior**

| Status | Meaning |
|---|---|
| `200 OK` | Event accepted into the queue |
| `503 Service Unavailable` | Queue full; the event was dropped |
| `500 Internal Server Error` | Queue channel closed unexpectedly |

---

## Current schema shape

`src/models/gsi_event.rs` currently models:

```text
GsiWebhookEvent
├─ hero: Hero
├─ abilities: Abilities
├─ items: Items
└─ map: Map
```

### `hero`

The model includes many fields, but the runtime currently reads this subset:

| GSI path | Current readers | What it drives |
|---|---|---|
| `hero.name` | `src/state/app_state.rs`, `src/actions/dispatcher.rs`, `src/gsi/handler.rs`, UI | Hero detection, dispatcher routing, debug logs |
| `hero.alive` | `src/gsi/handler.rs`, `src/actions/common.rs`, `src/actions/dispel.rs`, `src/actions/soul_ring.rs`, UI | Death/respawn logs, action gating, status display |
| `hero.health` | `src/actions/common.rs`, `src/actions/danger_detector.rs`, `src/gsi/handler.rs`, UI | Armlet logic, danger calculations, HP bars |
| `hero.health_percent` | `src/actions/common.rs`, `src/actions/danger_detector.rs`, `src/actions/auto_items.rs`, `src/actions/heroes/largo.rs`, `src/actions/soul_ring.rs` | Healing thresholds, danger checks, auto-abilities, Largo song choice, Soul Ring safety |
| `hero.max_health` | `src/actions/common.rs`, `src/actions/danger_detector.rs`, UI | HP percentage math and progress bars |
| `hero.mana` | UI | Mana bar text |
| `hero.mana_percent` | `src/actions/heroes/largo.rs`, `src/actions/soul_ring.rs` | Largo low-mana shutdown and Soul Ring gating |
| `hero.max_mana` | UI | Mana percentage display |
| `hero.stunned` | `src/actions/common.rs`, UI | Skip armlet toggles; status display |
| `hero.silenced` | `src/actions/dispel.rs`, UI | Silence dispel logic and status display |
| `hero.has_debuff` | `src/actions/heroes/huskar.rs` | Huskar Berserker Blood cleanse timing |
| `hero.aghanims_scepter` | `src/actions/heroes/largo.rs`, tests | Largo dual-song mode detection |
| `hero.aghanims_shard` | `src/actions/heroes/largo.rs` | Largo dual-song mode detection |
| `hero.level` | UI, tests | Status display and fixture assertions |
| `hero.respawn_seconds` | UI | Respawn countdown text |

Fields such as `hero.magicimmune`, `hero.break`, positions, talents, and buyback data are modeled but not currently consumed by runtime logic.

### `abilities`

| GSI path | Current readers | What it drives |
|---|---|---|
| `abilities.ability0.name` | `src/actions/heroes/largo.rs` | Detect whether Largo ultimate is active and which song occupies `Q` |
| `abilities.ability0`-`ability3` | `src/actions/heroes/huskar.rs` | Scan for `huskar_berserkers_blood` by ability name |
| `abilities.get_by_index(index)` | `src/actions/auto_items.rs` | Broodmother auto-abilities by configured slot index |
| `ability.can_cast` | `src/actions/heroes/huskar.rs`, `src/actions/auto_items.rs`, `src/actions/heroes/shadow_fiend.rs` | Ability readiness checks |
| `ability.cooldown` | `src/actions/heroes/huskar.rs`, `src/actions/auto_items.rs` | Additional readiness checks |
| `ability.level` | `src/actions/heroes/huskar.rs`, `src/actions/auto_items.rs` | Skip unlearned abilities |
| `abilities.ability5.can_cast` | `src/actions/heroes/shadow_fiend.rs` | Shadow Fiend standalone combo only fires when the ultimate is ready |

`ability.ultimate` exists in the schema but is not currently read by runtime code.

### `items`

`src/models/gsi_event.rs::Items::all_slots()` intentionally narrows most runtime checks to:

- `items.slot0`
- `items.slot1`
- `items.slot2`
- `items.slot3`
- `items.slot4`
- `items.slot5`
- `items.neutral0`

Those slots feed these behaviors:

| GSI path / field | Current readers | What it drives |
|---|---|---|
| `item.name` | `src/actions/common.rs`, `src/actions/dispatcher.rs`, `src/actions/dispel.rs`, `src/actions/soul_ring.rs`, `src/actions/auto_items.rs`, hero scripts, tests | Item presence, slot lookup, skip lists, fixture assertions |
| `item.can_cast` | shared actions, Soul Ring, Shadow Fiend, Broodmother, tests | Readiness checks |
| `item.cooldown` | `src/actions/auto_items.rs`, `src/actions/dispel.rs` | Readiness checks for auto-items and silence dispels |
| `item.charges` | tests | Covered by fixture assertions today; current runtime logic does not branch on charges directly |
| `item.passive` | `src/actions/dispatcher.rs` | Neutral-item discovery logging |
| `items.neutral0.name` | `src/actions/dispatcher.rs`, `src/actions/common.rs`, tests | Neutral discovery logging and neutral-item auto-use |

The model also includes `slot6`-`slot8`, `stash0`-`stash5`, and `teleport0`, but current action logic does not consult them.

### `map`

| GSI path | Current readers | What it drives |
|---|---|---|
| `map.clock_time` | none today | Present in the schema and fixtures, but not currently used by runtime logic |

---

## Where to edit when behavior is GSI-driven

| You need to change… | Start here |
|---|---|
| The JSON shape or field names | `src/models/gsi_event.rs` |
| The HTTP endpoint, queueing, or status codes | `src/gsi/server.rs`, `src/gsi/handler.rs` |
| What happens on every event before hero logic | `src/actions/dispatcher.rs` |
| Shared survivability from GSI stats/items | `src/actions/common.rs`, `src/actions/danger_detector.rs`, `src/actions/dispel.rs` |
| A hero-specific decision based on GSI | `src/actions/heroes/<hero>.rs` |
| A keyboard combo that also depends on GSI state | `src/actions/soul_ring.rs`, `src/input/keyboard.rs`, hero script |
| The docs that explain consumed fields | this page plus `docs/features/*.md` / `docs/heroes/*.md` |

---

## Test and fixture workflow

Current coverage lives in:

- `tests/gsi_handler_tests.rs`
- `tests/fixtures/huskar_event.json`
- `tests/fixtures/tiny_event.json`

Today those tests are schema/fixture smoke tests, not full end-to-end HTTP handler tests. They prove that:

- the payloads deserialize into `GsiWebhookEvent`
- expected hero/item fields exist
- representative fixtures stay in sync with the Rust model

When you add a new GSI-driven branch:

1. copy or extend a fixture in `tests/fixtures/`
2. deserialize it in `tests/gsi_handler_tests.rs`
3. assert the specific field(s) your logic depends on
4. if you changed the schema, update `src/models/gsi_event.rs` and this page together

See `docs/workflows/testing-and-debugging.md` for the test command flow.

---

## Debugging checklist

| Symptom | First places to look |
|---|---|
| Event counter stays at `0` | `src/gsi/server.rs`, Dota GSI target URL/port, `AppState.last_event` in UI |
| Hero is wrong or `None` | `hero.name`, `src/state/app_state.rs`, `src/actions/dispatcher.rs` |
| Shared healing / defensive item logic never fires | `item.name`, `item.can_cast`, `src/actions/common.rs`, `src/actions/danger_detector.rs` |
| Auto-items or silence dispels never fire | `item.name`, `item.can_cast`, `item.cooldown`, `src/actions/auto_items.rs`, `src/actions/dispel.rs` |
| Soul Ring / auto-items feel stale | `src/actions/dispatcher.rs` cache updates, `src/actions/soul_ring.rs`, `src/actions/auto_items.rs` |
| A hero script is not seeing its condition | the matching `src/actions/heroes/<hero>.rs` file plus the specific GSI field in this table |

For runtime debugging, pair this page with:

- `docs/workflows/testing-and-debugging.md`
- `docs/workflows/troubleshooting.md`
- `docs/features/keyboard-interception.md` for hybrid GSI + input flows
