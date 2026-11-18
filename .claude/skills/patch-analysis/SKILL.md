---
name: patch-analysis
description: Analyse a new Dota 2 gameplay patch — summarise the notes, forecast which heroes could go meta from measured win rates plus a per-hero patch rating, and list what in this repo the patch breaks. Use when the user shares patch notes, names a new patch (7.xx), or asks what changed or what will be meta.
---

# Patch analysis

The workflow and its pitfalls live in `docs/patch-analysis/README.md`. Read it first.
The previous patch's folder (for example `docs/patch-analysis/7.42/`) is the template
for `summary.md` and `deltas.csv`.

## Steps

1. `pwsh scripts/patch-analysis.ps1 -Patch <patch> -Notes`, then read
   `docs/patch-analysis/<patch>/notes.md` in full. It is long (7.42 was about 2,300
   lines). Read it in chunks.
2. Write `deltas.csv` (`hero,delta,reason`, −3..+3). Hero names must match Valve's hero
   list exactly; the score step warns about any it can't match.
3. `pwsh scripts/patch-analysis.ps1 -Patch <patch> -Measure -Score`. Use
   `-ReleaseUtc` if you know when the patch actually went live.
4. Write `summary.md`. Always include the repo-impact table: grep `src/` for every
   removed or renamed item and ability, and check each hero in `src/actions/heroes/`
   against its notes. Mention when `scripts/generate-mana-costs.ps1` needs a rerun.
5. Tell the user plainly which parts are measured (win rates) and which are your
   judgment (the patch rating). If the "after" window was skipped, say so and offer
   to re-measure once a few days of games exist.

Commit only when the user asks. Use a `docs:` Conventional Commit with no trailers.
