# Patch 7.42 analysis

| | |
|---|---|
| Valve stamp | 2026-11-18 08:00 UTC (live release came later the same day, Pacific time) |
| Scope | 128 heroes, 84 item entries, 83 neutral item entries |
| Full notes | [`notes.md`](notes.md) |
| Baseline data | [`data/`](data/): 7.41f, 2026-11-17 08:00 → 2026-11-18 20:00 UTC, 793k All Pick matches, 79k at Divine+ |
| Ranking | [`scored.csv`](scored.csv), from [`deltas.csv`](deltas.csv) |
| Post-patch data | Not yet. Re-run `-Measure -Score` with the real release time after a few days of games |

## 1. Global changes

**Hero stats**
- Base HP 120 → 200, but HP per Strength 22 → 20. Heroes under 40 Strength gain HP
  (about +40 at level 1); heroes over 40 lose it (−120 at 100 Strength). Laning is less
  lethal; huge Strength carries get slightly squishier late.
- Magic resistance per Intelligence 0.1% → 0.125% (8 Int per 1%).

**Spell Power Reduction.** Most magic and pure nukes deal less damage. Item spell amp,
AoE and cast range are cut: Kaya 10 → 6%, Aether +225 → +200, Shiva's and Gleipnir AoE
+75 → +60, Octarine 25 → 20%, Veil 10 → 8%. Long slows are shorter.

**Item Potency.** New attack stat that scales on-hit item effects, and illusions now
apply most of them. See [section 4](#4-item-potency).

**Economy and experience**
- Passive gold is a flat 90/min (used to climb to 111 by 60:00).
- Kill bounty 125 + 8×level → 100 + 10×level: cheaper before about level 13, richer after.
- Tower, barracks and Roshan gold down.
- Everyone gains 50 XP/min, even dead. Level 30 needs 66,600 XP instead of 64,400; the
  extra sits in levels 5–18.

**Objectives**
- Tormentor spawns at 15:00 (was 20:00). It always drops the new Greater Horn of Battle,
  which sends a mega-creep wave down every lane, and drops Aghanim's Shard from its
  second death.
- Roshan's respawn shrinks by 1 minute per kill, down to 5 minutes.
- Glyph has 2 charges. After 30:00, siege creeps come every 2.5 minutes.

**Other mechanics**
- Spells that pierce debuff immunity (Reverse Polarity, Dismember, Duel and so on) now
  respect BKB's magic resistance.
- Leashes no longer block movement spells cast after debuff immunity.
- Invisibility rune adds Disguise: True Sight protection while away from enemy heroes
  and buildings.
- Couriers, wards and summons share one pip-health system; melee heroes deal double
  pips to couriers.

**Map.** Twin Gates move to the top and bottom edges. Mid Tier 1 and Tier 2 towers move
so both sides' distances match. The Dire fountain moves about 300 units closer to the
Ancient. Creeps pulled out of lane slow down by up to 25%, except near your safe-lane
Tier 1 tower.

## 2. Items

| New item | Cost | What it does |
|---|---|---|
| Sabatons of Mordiggian | 5950 | Phase Boots + Armlet. Toggle drains HP for +40 damage, +30 Str, +5 armor and slow resistance |
| Skypiercer | 6000 | Halberd + Sange + Talisman. +35 Str; Disarm recharges 2.25× faster near enemies |
| Starforge | 6000 | Meteor Hammer + Ultimate Orb. Meteor also deals 15% max HP; every disable you land burns |
| Caduceus | 5400 | Force Staff + Phylactery. The empowered spell also heals and hastes allies |
| Ristul's Devotion | 3800 | Vladmir's + Splintmail + Voodoo Mask. Aura: 20% lifesteal, 18% damage |
| Bloodslinger | 925 | Upgraded Blood Grenade |

- **Removed:** Khanda, Helm of the Overlord, Javelin. **Back:** Quarterstaff (900).
- MKB, Mask of Madness and Butterfly build from Quarterstaff. Butterfly costs 5000 and
  loses its +20% base attack speed.
- Abyssal Blade and Harpoon build from Dragon Lance, which now gives melee heroes +25
  range.
- Heaven's Halberd is a cheap Strength item: 2400 gold, +12 Str.
- Sange, Yasha and Kaya cost 2000 each; their combinations cost 4000.
- Pipe's barrier and Crimson Guard's Guard can be dispelled; Crimson adds a 2-armor aura.
- **Neutral items:** 4 tiers at 0, 15, 30 and 50 minutes. Each tier is 4 fixed pools of
  3, one item per pool, all visible from the start. Evolved drops to +20 primary
  attribute but adds +20% Item Potency.

## 3. Hero highlights

- **Reworks:** Chaos Knight (new innate, a global charge with his illusions), Death
  Prophet (attacks send healing ghosts; range 600 → 500), Leshrac (Nihilism is an AoE
  disarm plus self-dispel), Chen (Zealot), Abaddon (Withering Mist, Mist Coil, new
  Scepter), Lycan and Nyx (new innates), Pugna (Scepter: second cast of every spell),
  Medusa (Gorgon's Grasp is magical and hits harder).
- **Meepo:** Universal now. MegaMeepo removed; Dig moves from Shard to Scepter; new Shard
  ability Fling.
- **Juggernaut:** Swiftslash removed; Blade Fury applies a basic dispel instead of a
  strong one; new Scepter lets him attack while spinning.
- **New abilities:** Beastmaster Scepter Ancient Thunderhide (Drums of Slom removed),
  Undying Shard Zombie Bite, Brewmaster's Fire brewling Cinder Dash, Spirit Bear Scepter
  Summon Lone Druid.
- **Nuke nerfs:** Laguna −70, Finger −50, Sun Strike −80 at max, Mystic Flare −150,
  Sonic Wave −40, Chain Frost −50 per bounce, Black Hole −20 DPS, Dismember Str
  multiplier −0.2.

## 4. Item Potency

### The rule

Every attack carries an Item Potency value. It multiplies the *strength* of certain
on-hit item effects, not the attack's damage:

| Item | What potency scales |
|---|---|
| Desolator, Orb of Blight | Armor reduction |
| Eye of Skadi, Orb of Frost, Orb of Corrosion | Move slow, attack slow, healing reduction (all three now pierce debuff immunity) |
| Diffusal Blade, Disperser | Mana burned |
| Skull Basher, Abyssal Blade | Bash chance (Basher does not work on illusions at all) |
| Maelstrom, Mjollnir | Chain Lightning chance |
| Monkey King Bar | Pierce chance (illusions instead get a separate 30 magic damage) |
| Mage Slayer | Spell damage reduction |
| Witch Blade, Parasma | Slow; Parasma's magic resistance reduction |
| Silver Edge | Break duration |
| Serrated Shiv, Vile Phial, Mind Breaker (neutrals) | Proc chance, healing reduction, silence duration |

Lifesteal, crits and raw damage are not on the list.

Three rules decide the number:

1. A hero's normal attack is **100%**.
2. Illusions default to **20%**, and they now apply most on-hit items. Before 7.42 they
   applied almost none.
3. **When several sources apply the same debuff, only the highest potency counts.**
   Potency does not stack.

Worked example with Desolator (−6 armor):

| Attacker | Potency | Armor removed |
|---|---|---|
| Hero with Evolved enchantment | 120% | 7.2 |
| Hero | 100% | 6 |
| Chaos Knight Phantasm, Vengeful Scepter illusion, Riftshadow Prism illusion | 100% | 6 |
| Dark Seer replica | 60% | 3.6 |
| Spectre Haunt | 40% | 2.4 |
| Manta illusion | 35% | 2.1 |
| Phantom Lancer or Naga illusion (default) | 20% | 1.2 |

### What rule 3 means in practice

- **Debuffs (armor, slows, heal cut) don't add up.** Five PL illusions plus PL on one
  target still remove 6 armor, the hero's value. Illusions add value when they reach
  targets the hero isn't hitting, or keep a debuff up while the hero is elsewhere.
- **Chance procs do add up.** Every attack rolls on its own, so more attacks means more
  procs, each at a scaled-down chance. Volume heroes get the most from Maelstrom,
  Mjollnir and Serrated Shiv.
- **Per-hit resources add up.** Each illusion hit burns its own (scaled) Diffusal mana,
  so a swarm drains mana again.

### Per hero

Illusions and replicas:

| Hero / source | Potency | Comment |
|---|---|---|
| Chaos Knight: Phantasm | **100%** | Every Phantasm is a full Skadi/Desolator/Diffusal carrier. Paired with the new charge innate, this is the biggest winner |
| Chaos Knight: Chaos Bolt Shard illusion | 25% | Also loses its 15% damage penalty |
| Vengeful Spirit: Scepter illusion | 100% | |
| Riftshadow Prism (tier 4 neutral) | 100% | Refract is now a strong illusion. Any hero can use it |
| Dark Seer: Wall of Replica, Normal Punch | 60% | |
| Grimstroke: Dark Portrait | 50% | |
| Illusion rune | 40% | |
| Spectre: Haunt | 40% | Haunt can now carry real Desolator/Skadi/Diffusal pressure |
| Manta Style | 35% | Benefits every Manta carrier |
| Spectre: Shadow Step illusion | 25% | |
| Terrorblade: Reflection | 25% | |
| Ringmaster: Funhouse Mirror | 25% | |
| Phantom Lancer, Naga Siren, Terrorblade Conjure Image, other illusions | 20% | Not listed in the notes, so the default applies. The gain is that they apply effects at all |

Meepo clones and Arc Warden's Tempest Double are real heroes, not illusions, so they
should attack at 100%. The notes don't say so directly.

Hero attacks above 100%:

| Hero | Potency |
|---|---|
| Sniper: Assassinate | 120 / 135 / 150% |
| Riki: Backstab (from behind) | 110%, +10% per 15 hero levels |
| Void Spirit: Astral Step | 125% |
| Mirana: Celestial Quiver | 120% |
| Weaver: Geminate extra attacks | 115% |
| Anyone with the Evolved enchantment | +20% on every attack |

Hero attacks below 100% (attacks a spell makes on your behalf):

| Hero | Potency |
|---|---|
| Ember Spirit: Sleight of Fist | 80% |
| Abaddon: Scepter Mist Coil attacks | 80% |
| Windranger: Focus Fire | 75% |
| Juggernaut: Scepter Blade Fury attacks | 75% |
| Gyrocopter: Side Gunner | 75% |
| Pangolier: Swashbuckle | 60% (now 4 slashes) |
| Sand King: Stinger | 60% |
| Gyrocopter: Flak Cannon | 50%, and it now procs attack modifiers except lifesteal and crit |
| Kez: Falcon Rush echo attacks | 30 / 35 / 40 / 45%, matching their damage |
| Luna: Moon Glaives | Bounces are full attacks; potency drops 40% per bounce |

**Who it helps most:** Chaos Knight, then Spectre, Phantom Lancer, Naga Siren and any
Manta carrier, Dark Seer, and Vengeful Spirit with Scepter. On-hit items in general get
better: Diffusal, Skadi, Corrosion, Mjollnir and Mage Slayer on illusion heroes.
Ember Spirit's Sleight of Fist is a small hidden nerf: its on-hit effects now apply at
80%.

## 5. Meta forecast

**Method.** Score = baseline + 1.2 × patch rating.

- Baseline = 0.6 × (Divine+ win rate − 50) + 0.4 × (all-rank win rate − 50), from the
  7.41f window.
- Patch rating = −3 to +3 per hero, in `deltas.csv`, with a reason on every row. **This
  part is judgment, not measurement.**

Top of `scored.csv`:

| # | Hero | 7.41f WR | Divine+ WR | Pick % | Patch | Why |
|---|---|---|---|---|---|---|
| 1 | Visage | 52.9 | 55.8 | 1.7 | +0.5 | Already a high-rank monster, small buff |
| 2 | **Chaos Knight** | 51.2 | 51.2 | 4.5 | +3 | Biggest buff in the patch (Item Potency and the new innate) |
| 3 | **Leshrac** | 51.3 | 55.4 | 2.3 | +0.5 | Nihilism rework; early Tormentor and Horn of Battle reward pushing |
| 4 | **Wraith King** | 55.0 | 52.9 | 14.5 | +0.5 | Best public win rate, plus a longer stun and earlier Bone Guard |
| 5 | Riki | 53.0 | 53.5 | 6.5 | +0.5 | Backstab potency bonus |
| 6 | Troll Warlord | 52.2 | 54.0 | 3.0 | +0.5 | Battle Trance gives status resistance |
| 7 | **Spectre** | 53.0 | 52.3 | 9.2 | +1 | Haunt 40% potency |
| 8 | **Phantom Lancer** | 52.4 | 52.5 | 11.9 | +1 | Diffusal on illusions again |
| 9 | Enigma | 51.0 | 57.1 | 4.3 | −1 | Nerfed, but the best Divine+ win rate in the game |
| 10 | Brewmaster | 50.7 | 53.6 | 2.1 | +0.5 | Fire brewling's Cinder Dash pull |

**Breakout candidates** (average or weak in 7.41f, big reworks): Naga Siren, Abaddon,
Venomancer, Lone Druid, Chen, Death Prophet. Supports that look good: Disruptor, Dazzle,
Phoenix, Pugna.

**Likely losers:** magic-burst heroes (Lina, Lion, Skywrath, Zeus, Queen of Pain);
Outworld Destroyer (nerfed on every ability, from 53%); Juggernaut; Pudge; Bounty
Hunter; Nyx (loses Mana Burn); Meepo (still 54% at Divine+, but no MegaMeepo and Dig
moved to Scepter). Shadow Fiend (43%) and Nature's Prophet (43%) stay at the bottom.

**Overall read.** Attack and illusion heroes beat magic burst. An earlier Tormentor plus
Horn of Battle favours teams that fight and push together in the mid-game. Flat passive
gold and a tier 4 neutral at 50:00 weaken late-game hard carries.

## 6. Impact on this repo

| Area | Change | Where |
|---|---|---|
| Meepo | `meepo_megameepo` no longer exists; Dig is Scepter-only | `src/actions/heroes/meepo.rs`, `meepo_state.rs`, `meepo_macro.rs`, Meepo config in `src/config/settings.rs` |
| Mana costs | Khanda, Helm of the Overlord, Javelin and MegaMeepo still listed; new items missing | Regenerate with `scripts/generate-mana-costs.ps1` |
| Armlet | Sabatons of Mordiggian is a new HP-draining toggle built from Armlet; the armlet logic won't recognise it | `src/actions/armlet.rs` |
| Morphling | Ebb and Flow is now fully disabled by Break | `src/actions/heroes/morphling.rs` |
| Largo | Amphibian Rhapsody cooldown 2s → 1.5s | `src/actions/heroes/largo.rs`; re-run `examples/largo_song_sync_control.rs` |
| Earth Spirit, Ember Spirit, Slark, Huskar, Mirana, Magnus, Tiny, SF, PL, OD, Legion, Broodmother, Snapfire, Invoker | Number changes only, or ability changes these scripts don't drive | Check each hero doc in `docs/heroes/` |
