//! Verifies the STRATZ integration against the live API.
//!
//! The unit tests pin the parsers against fixtures, which proves the parsing
//! but not that STRATZ's schema matches. Only a real call does that. This
//! builds the full dataset, saves the cache, and prints sanity checks that
//! would catch a query returning structurally valid nonsense.
//!
//! Needs a token:
//!   $env:STRATZ_TOKEN = "eyJ..."
//!   cargo run --release --example stratz_probe
//!
//! Options:
//!   --advise      rank picks for a sample draft using the built dataset
//!   --cache-only  skip the fetch and report on the cached dataset

use dota2_scripts::stratz::advice;
use dota2_scripts::stratz::advisor::{recommend, AdviceWeights, DraftContext};
use dota2_scripts::stratz::client::StratzClient;
use dota2_scripts::stratz::dataset::{self, StratzDataset, NUM_POSITIONS};
use dota2_scripts::stratz::fetch::{build_dataset, Bracket};

fn cache_path() -> std::path::PathBuf {
    std::path::PathBuf::from(".cache/stratz_dataset.bin")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cache_only = args.iter().any(|a| a == "--cache-only");

    let dataset = if cache_only {
        dataset::load(&cache_path()).expect("no cached dataset; run without --cache-only first")
    } else {
        let token = StratzClient::resolve_token("");
        if token.is_empty() {
            eprintln!("Set STRATZ_TOKEN first.");
            std::process::exit(1);
        }
        let mut client = StratzClient::new(token);
        let bracket = Bracket::divine_immortal();
        let started = std::time::Instant::now();

        let mut last_stage = String::new();
        let mut on_progress = |done: usize, total: usize, stage: &str| {
            if stage != last_stage {
                println!("  [{stage}]");
                last_stage = stage.to_string();
            }
            if stage == "matchups" && done % 20 == 0 && done > 0 {
                println!("      {done}/{total}");
            }
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        println!("Building dataset (about a minute at the free rate limit)...");
        let d = match build_dataset(&mut client, &bracket, now, &mut on_progress) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("FAILED: {e}");
                std::process::exit(1);
            }
        };
        println!("Built in {:.1}s", started.elapsed().as_secs_f32());
        dataset::save(&d, &cache_path()).expect("save cache");
        println!("Cached -> {}", cache_path().display());
        d
    };

    report(&dataset);

    if args.iter().any(|a| a == "--advise") {
        advise_sample(&dataset);
    }
}

fn report(d: &StratzDataset) {
    let n = d.len();
    println!("\n=== dataset ===");
    println!("heroes:  {n}");
    println!("bracket: {}", d.bracket);

    // Coverage: a query that silently returned nothing shows up here as a
    // structurally valid but empty matrix.
    let vs_filled = d.vs_matches.iter().filter(|&&m| m > 0).count();
    let with_filled = d.with_matches.iter().filter(|&&m| m > 0).count();
    let possible = n * (n - 1);
    println!(
        "counter coverage: {:.1}%  synergy coverage: {:.1}%",
        100.0 * vs_filled as f32 / possible as f32,
        100.0 * with_filled as f32 / possible as f32
    );

    let mut samples: Vec<u32> = d.vs_matches.iter().copied().filter(|&m| m > 0).collect();
    samples.sort_unstable();
    if !samples.is_empty() {
        println!(
            "matchup sample: median {} games, min {}, max {}",
            samples[samples.len() / 2],
            samples[0],
            samples[samples.len() - 1]
        );
    }

    let base_min = d.base_win_rate.iter().cloned().fold(f32::MAX, f32::min);
    let base_max = d.base_win_rate.iter().cloned().fold(f32::MIN, f32::max);
    let base_mean = d.base_win_rate.iter().sum::<f32>() / n as f32;
    println!("baseline win rate: min {base_min:.4}  mean {base_mean:.4}  max {base_max:.4}");
    // Aggregate win rate across all heroes must sit near 50% -- if it does
    // not, the baseline is being computed from the wrong population.
    if !(0.45..=0.55).contains(&base_mean) {
        println!("  !! mean baseline is implausible; check the baseline source");
    }

    let pos_covered = (0..n)
        .filter(|&h| (0..NUM_POSITIONS).any(|p| d.position_share_of(h, p) > 0.0))
        .count();
    println!("heroes with position data: {pos_covered}/{n}");

    // Spot-check a hero everyone knows the shape of.
    if let Some(sf) = d.index_of_slug("nevermore") {
        println!("\n--- Shadow Fiend ---");
        println!("baseline: {:.4}", d.base_win_rate[sf]);
        for p in 0..NUM_POSITIONS {
            let share = d.position_share_of(sf, p);
            if share > 0.01 {
                let wr = d
                    .position_win_rate_of(sf, p)
                    .map(|w| format!("{w:.4}"))
                    .unwrap_or_else(|| "-".into());
                println!("  pos {}: {:.1}% of games, wr {wr}", p + 1, share * 100.0);
            }
        }
        print_extremes(d, sf);
    }
}

/// The heroes a given hero most and least wants to face. If these look
/// arbitrary, the advantage matrix is wrong even though it is well-formed.
fn print_extremes(d: &StratzDataset, hero: usize) {
    let n = d.len();
    let mut scored: Vec<(f32, u32, &str)> = (0..n)
        .filter(|&j| j != hero)
        .map(|j| {
            let (adv, m) = d.advantage_of(hero, j);
            (adv, m, d.hero(j).map(|h| h.display_name.as_str()).unwrap_or("?"))
        })
        .filter(|(_, m, _)| *m > 500)
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    println!("  strongest against:");
    for (adv, m, name) in scored.iter().take(5) {
        println!("    {name:<22} {:+.3} ({m} games)", adv);
    }
    println!("  weakest against:");
    for (adv, m, name) in scored.iter().rev().take(5) {
        println!("    {name:<22} {:+.3} ({m} games)", adv);
    }
}

fn advise_sample(d: &StratzDataset) {
    // A plausible enemy lineup, to see whether the advice reads sensibly.
    let enemy_slugs = ["nevermore", "crystal_maiden", "axe", "juggernaut", "lion"];
    let enemies: Vec<usize> = enemy_slugs.iter().filter_map(|s| d.index_of_slug(s)).collect();
    println!(
        "\n=== advice vs {} ===",
        enemy_slugs.join(", ")
    );

    for position in [0usize, 4] {
        let ctx = DraftContext {
            allies: Vec::new(),
            enemies: enemies.clone(),
            position: Some(position),
        };
        let picks = recommend(d, &ctx, &AdviceWeights::default(), 8);
        println!("\n-- as position {} --", position + 1);
        for (rank, p) in picks.iter().enumerate() {
            let against = p
                .best_against
                .as_ref()
                .map(|(name, v)| format!("  (best vs {name} {v:+.3})"))
                .unwrap_or_default();
            println!(
                "  {:>2}. {:<22} score {:+.3}  counter {:+.3}{}",
                rank + 1,
                p.display_name,
                p.score,
                p.counter,
                against
            );
        }
    }
    let _ = advice::DraftAdvice::default();
}
