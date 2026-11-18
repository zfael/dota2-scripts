# Patch analysis

One folder per Dota gameplay patch: what changed, which heroes could go meta, and what
in this repo needs updating.

```
docs/patch-analysis/<patch>/
  notes.md      full patch notes, ids resolved to names       (generated)
  deltas.csv    hero,delta,reason: -3..+3 rating per hero     (hand-written)
  scored.csv    baseline win rates + rating -> meta score     (generated)
  summary.md    the write-up                                  (hand-written)
  data/         per-hero picks/wins and windows.json          (generated)
```

## Workflow

1. **Notes:** `pwsh scripts/patch-analysis.ps1 -Patch 7.43 -Notes`
2. **Read** `notes.md` and write `deltas.csv`: one row for every hero the patch
   meaningfully moved, with the reason. Leave unchanged heroes out; they score 0.
   Fold the systemic changes into each hero's number (for 7.42: illusion Item Potency,
   Spell Power Reduction).
3. **Measure and score:** `pwsh scripts/patch-analysis.ps1 -Patch 7.43 -Measure -Score`
4. **Write** `summary.md`: global changes, items, hero highlights, the meta forecast
   from `scored.csv`, and a repo-impact table covering every hero in
   `src/actions/heroes/` and every item we automate.
5. **Re-measure** a few days after release with the real release time, so the "after"
   window fills in:
   `pwsh scripts/patch-analysis.ps1 -Patch 7.43 -Measure -Score -ReleaseUtc <when it went live>`.
   `scored.csv` then gains `after_wr`, `d_wr` and `z` columns. Update `summary.md` with
   what the data confirmed or refuted.

## Things that bite

- **Valve's patch timestamp is not the release time.** It is a date stamp, often 08:00
  UTC, and the patch reaches matchmaking hours later (7.40: about 17 hours). With the
  default split, the "after" window is mostly old-patch games and every hero looks
  unchanged. The script skips the "after" window until `-MinAfterHours` (24) have
  passed, but it cannot know the real release time. Pass `-ReleaseUtc`.
- **OpenDota's explorer times out** on long windows. Queries run in `-ChunkHours` (12)
  slices and are summed.
- **`deltas.csv` is opinion.** Keep a reason on every row so a later re-measure can say
  which calls were wrong.
- Downloads are cached under `.cache/patch-analysis/<patch>/` (gitignored). Delete that
  folder to force a fresh fetch if Valve edits the notes.

## Sources

- Valve patch feed: `https://www.dota2.com/datafeed/patchnotes?version=<patch>&language=english`,
  plus `patchnoteslist`, `herolist`, `itemlist` and `abilitylist`.
- OpenDota explorer, `public_matches`, All Pick (`game_mode = 22`). The high-rank split
  is `avg_rank_tier >= 70` (Divine 0+).
