//! Does the cached STRATZ dataset already know what is "meta"?
//!
//! Answering three questions before any of this reaches `src/`:
//!
//! 1. **Is the local hero mapping right?** The draft reader emits Dota's
//!    internal slugs (`necrolyte`, `zuus`, `vengefulspirit`); the advice is
//!    keyed by STRATZ ids. If those resolve to the wrong hero the advice is
//!    confidently wrong, so every slug from a real draft is resolved here and
//!    printed with the id and name STRATZ has for it.
//!
//! 2. **Can pick rate be derived from what we already cache?** `matchUp` gives
//!    one `vs` row per opponent, so a hero's row sums to 5x the games it
//!    played (five enemies per game). With ten heroes per match that makes
//!    `pick_rate(i) = 10 * rowsum(i) / total_rowsum` — no extra request, no
//!    dataset format change. The check that this is real and not arithmetic
//!    that merely looks plausible: the rates must sum to 10.0, and the top of
//!    the list must be heroes a Dota player recognises as currently popular.
//!
//! 3. **What threshold means "meta"?** Printed as a distribution rather than
//!    guessed, then applied to a real draft to see what a toggle would remove.
//!
//! Read-only against `.cache/stratz_dataset.bin`:
//!   cargo run --release --example stratz_meta_probe

use dota2_scripts::stratz::advisor::{recommend, AdviceWeights, DraftContext, META_PICK_RATE_MULTIPLE};
use dota2_scripts::stratz::dataset::{self, StratzDataset, NUM_POSITIONS};

/// The lineup from the session that prompted this, exactly as the draft
/// reader spelled it.
const ALLIES: [&str; 5] = [
    "obsidian_destroyer",
    "sven",
    "axe",
    "witch_doctor",
    "lion",
];
const ENEMIES: [&str; 5] = [
    "necrolyte",
    "drow_ranger",
    "zuus",
    "earthshaker",
    "vengefulspirit",
];

fn main() {
    let path = std::path::PathBuf::from(".cache/stratz_dataset.bin");
    let d = match dataset::load(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("no usable cache at {}: {e}", path.display());
            eprintln!("run `cargo run --release --example stratz_probe` first");
            std::process::exit(1);
        }
    };
    println!(
        "dataset: {} heroes, bracket {}, built {}",
        d.len(),
        d.bracket,
        d.built_at
    );

    check_mapping(&d);
    let picks = pick_rates(&d);
    report_popularity(&d, &picks);
    report_thresholds(&d, &picks);
    report_position_meta(&d);
    report_advice(&d);
    explain_top_pick(&d);
}

// --- 1. mapping --------------------------------------------------------------

fn check_mapping(d: &StratzDataset) {
    println!("\n=== 1. slug -> STRATZ mapping ===");
    println!("{:<22} {:>5}  {:<22} {}", "reader slug", "id", "STRATZ name", "note");
    for slug in ALLIES.iter().chain(ENEMIES.iter()) {
        match d.index_of_slug(slug) {
            Some(i) => {
                let h = d.hero(i).unwrap();
                // A mismatch between the two spellings is exactly what the
                // separator-insensitive lookup exists to absorb; worth seeing.
                let note = if h.slug == *slug { "" } else { "resolved via normalisation" };
                println!("{slug:<22} {:>5}  {:<22} {note}", h.id, h.display_name);
            }
            None => println!("{slug:<22} {:>5}  {:<22} !! UNRESOLVED", "-", "-"),
        }
    }

    let gaps = d.heroes_without_matchups();
    println!("\nheroes with no matchup row at all: {} {:?}", gaps.len(), gaps);

    // Every hero whose slug does not title-case into its real name. The
    // Lineup panel currently prints the title-cased slug, which is how
    // "Necrophos" ends up on screen as "Necrolyte" three inches from the
    // advice that calls it Necrophos. This is the exact set the UI needs.
    println!("\nslugs that do not title-case into their name (UI needs these):");
    for h in &d.heroes {
        let title: String = h
            .slug
            .split('_')
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        if title != h.display_name {
            println!("  {:<22} => {}", h.slug, h.display_name);
        }
    }
}

// --- 2. pick rate ------------------------------------------------------------

/// Share of matches each hero appears in — now `dataset::pick_rates`, kept
/// here as the thing this probe checks rather than a second implementation.
fn pick_rates(d: &StratzDataset) -> Vec<Option<f32>> {
    d.pick_rates(None)
}

fn report_popularity(d: &StratzDataset, picks: &[Option<f32>]) {
    println!("\n=== 2. derived pick rate ===");
    let sum: f32 = picks.iter().flatten().sum();
    println!("sum of pick rates: {sum:.3}  (must be ~10.0 — ten heroes per match)");

    let mut ranked: Vec<(usize, f32)> = picks
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.map(|v| (i, v)))
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    println!("\ntop 25 by pick rate (the list to eyeball against the real meta):");
    for (rank, (i, rate)) in ranked.iter().take(25).enumerate() {
        println!(
            "  {:>2}. {:<22} {:>5.2}% of games   wr {:>5.2}%",
            rank + 1,
            d.hero(*i).map(|h| h.display_name.as_str()).unwrap_or("?"),
            rate * 100.0,
            d.base_win_rate[*i] * 100.0
        );
    }
    println!("\nbottom 8:");
    for (i, rate) in ranked.iter().rev().take(8).rev() {
        println!(
            "      {:<22} {:>5.2}% of games   wr {:>5.2}%",
            d.hero(*i).map(|h| h.display_name.as_str()).unwrap_or("?"),
            rate * 100.0,
            d.base_win_rate[*i] * 100.0
        );
    }
}

fn report_thresholds(d: &StratzDataset, picks: &[Option<f32>]) {
    println!("\n=== 3. where to cut ===");
    let mut rates: Vec<f32> = picks.iter().flatten().copied().collect();
    rates.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = rates.len() as f32;
    let mean = rates.iter().sum::<f32>() / n;
    let pct = |q: f32| rates[((rates.len() as f32 - 1.0) * q) as usize];
    println!(
        "min {:.2}%  p25 {:.2}%  median {:.2}%  mean {:.2}%  p75 {:.2}%  p90 {:.2}%  max {:.2}%",
        rates[0] * 100.0,
        pct(0.25) * 100.0,
        pct(0.50) * 100.0,
        mean * 100.0,
        pct(0.75) * 100.0,
        pct(0.90) * 100.0,
        rates[rates.len() - 1] * 100.0
    );

    // Candidate rules, judged by how many heroes survive. A meta filter that
    // keeps 60 heroes has not filtered anything a drafter cares about; one
    // that keeps 10 throws away real picks.
    println!("\nsurvivors by rule (out of {} with data, {} heroes total):", rates.len(), d.len());
    for mult in [1.0f32, 1.25, 1.5, 2.0] {
        let kept = rates.iter().filter(|&&r| r >= mean * mult).count();
        println!("  pick rate >= {mult:.2}x mean ({:.2}%): {kept} heroes", mean * mult * 100.0);
    }
    for abs in [0.05f32, 0.08, 0.10, 0.12, 0.15] {
        let kept = rates.iter().filter(|&&r| r >= abs).count();
        println!("  pick rate >= {:.0}% of games: {kept} heroes", abs * 100.0);
    }
}

/// Pick rate for a hero *in one position*: its overall rate scaled by the
/// share of its own games played there.
fn report_position_meta(d: &StratzDataset) {
    println!("\n=== 4. meta per position ===");
    for p in 0..NUM_POSITIONS {
        let rates = d.pick_rates(Some(p));
        let mut ranked: Vec<(usize, f32)> = rates
            .iter()
            .enumerate()
            .filter_map(|(i, r)| r.map(|v| (i, v)))
            .filter(|(_, r)| *r > 0.0)
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let named: Vec<String> = ranked
            .iter()
            .take(10)
            .map(|(i, r)| {
                format!(
                    "{} {:.1}%",
                    d.hero(*i).map(|h| h.display_name.as_str()).unwrap_or("?"),
                    r * 100.0
                )
            })
            .collect();
        println!("  pos {}: {}", p + 1, named.join(", "));

        // The same relative rule has to work per role, where every rate is
        // ~1/5 of the overall one, or the toggle would empty the list the
        // moment a role is picked.
        let rates: Vec<f32> = ranked.iter().map(|(_, r)| *r).collect();
        let mean = rates.iter().sum::<f32>() / rates.len() as f32;
        let survivors: Vec<String> = [1.0f32, 1.25, 1.5, 2.0]
            .iter()
            .map(|m| format!("{m:.2}x: {}", rates.iter().filter(|&&r| r >= mean * m).count()))
            .collect();
        println!(
            "         mean {:.2}% over {} played there — survivors {}",
            mean * 100.0,
            rates.len(),
            survivors.join(", ")
        );
    }
}

// --- 4. effect on real advice ------------------------------------------------

fn indices(d: &StratzDataset, slugs: &[&str]) -> Vec<usize> {
    slugs.iter().filter_map(|s| d.index_of_slug(s)).collect()
}

fn report_advice(d: &StratzDataset) {
    println!("\n=== 5. what a meta toggle would change ===");
    // The draft that prompted this, minus our own picks, so there is actually
    // something to suggest.
    let enemies = indices(d, &ENEMIES);
    let allies = indices(d, &ALLIES[..2]);

    println!(
        "(meta cut = {META_PICK_RATE_MULTIPLE}x the mean pick rate, as shipped in advisor.rs)"
    );

    for position in [1usize, 4] {
        // The role-aware rates the filter itself reads, so "meta?" below is
        // the shipped rule and not a restatement of it.
        let role_rates = d.pick_rates(Some(position));
        let known: Vec<f32> = role_rates.iter().flatten().copied().collect();
        let cut = known.iter().sum::<f32>() / known.len() as f32 * META_PICK_RATE_MULTIPLE;

        let ctx = DraftContext {
            allies: allies.clone(),
            enemies: enemies.clone(),
            position: Some(position),
            meta_only: false,
        };
        let all = recommend(d, &ctx, &AdviceWeights::default(), 60);
        println!("\n-- position {} --", position + 1);
        println!("{:<4} {:<22} {:>7} {:>8} {:>8} {:>9}  {}", "", "hero", "score", "counter", "wr", "pick%", "meta?");
        for (rank, s) in all.iter().take(12).enumerate() {
            let rate = role_rates.get(s.hero_index).copied().flatten();
            let meta = match rate {
                Some(r) if r >= cut => "yes",
                Some(_) => "no",
                None => "unknown",
            };
            println!(
                "{:<4} {:<22} {:>+7.3} {:>+8.3} {:>7.1}% {:>8}  {meta}",
                format!("{}.", rank + 1),
                s.display_name,
                s.score,
                s.counter,
                s.position_win_rate.unwrap_or(f32::NAN) * 100.0,
                rate.map(|r| format!("{:.2}%", r * 100.0)).unwrap_or_else(|| "-".into()),
            );
        }
        // What the toggle actually produces, through the same code path the
        // app uses.
        let meta_ctx = DraftContext { meta_only: true, ..ctx.clone() };
        let kept = recommend(d, &meta_ctx, &AdviceWeights::default(), 12);
        println!(
            "  meta-only top 12: {}",
            kept.iter()
                .map(|s| format!(
                    "{} ({:.1}%)",
                    s.display_name,
                    s.pick_rate.unwrap_or(0.0) * 100.0
                ))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}

// --- 5. explaining one pick --------------------------------------------------

/// Every number behind the top suggestion, which is what the UI has to be able
/// to show if "why is this ranked first" is to be answerable.
fn explain_top_pick(d: &StratzDataset) {
    println!("\n=== 6. anatomy of one suggestion ===");
    let enemies = indices(d, &ENEMIES);
    let allies = indices(d, &ALLIES[..2]);
    let ctx = DraftContext {
        allies: allies.clone(),
        enemies: enemies.clone(),
        position: Some(1),
        meta_only: false,
    };
    let picks = recommend(d, &ctx, &AdviceWeights::default(), 1);
    let Some(top) = picks.first() else { return };
    let h = top.hero_index;
    println!(
        "{} — score {:+.3} = base {:+.3}? + counter {:+.3} + synergy {:+.3}",
        top.display_name,
        top.score,
        top.score - top.counter - top.synergy,
        top.counter,
        top.synergy
    );

    println!("\n  vs each enemy:");
    for &e in &enemies {
        let (adv, m) = d.advantage_of(h, e);
        let rel = m as f32 / (m as f32 + 50.0);
        println!(
            "    {:<22} {:+.3} over {:>6} games -> contributes {:+.3}",
            d.hero(e).map(|x| x.display_name.as_str()).unwrap_or("?"),
            adv,
            m,
            adv * rel
        );
    }
    println!("\n  with each ally:");
    for &a in &allies {
        let (syn, m) = d.synergy_of(h, a);
        let rel = m as f32 / (m as f32 + 50.0);
        println!(
            "    {:<22} {:+.3} over {:>6} games -> contributes {:+.3}",
            d.hero(a).map(|x| x.display_name.as_str()).unwrap_or("?"),
            syn,
            m,
            syn * rel
        );
    }

    // How often the "best vs" line would name the same enemy for everyone —
    // the UI currently shows it for every row, and a column that says the same
    // thing twelve times is a column carrying no information.
    let all = recommend(d, &ctx, &AdviceWeights::default(), 12);
    let mut tally = std::collections::HashMap::<String, usize>::new();
    for s in &all {
        if let Some((name, _)) = &s.best_against {
            *tally.entry(name.clone()).or_default() += 1;
        }
    }
    let mut tally: Vec<_> = tally.into_iter().collect();
    tally.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    println!("\n  \"best vs\" across the top 12: {tally:?}");
}
