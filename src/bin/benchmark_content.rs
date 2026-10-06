/*  This file is part of the OmniDiff code diffing tool.
 *
 *  Copyright (C) 2026 Marko Ivankovic
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

//! Speed and accuracy per content family, the README's Fast and Accurate figures for content
//! other than code (`make benchmark-content`).
//!
//! **Speed** is over the sampled pairs in `src/test/data/samples/` (`sample_test_diffs --content`,
//! then `materialize_test_diffs`), grouped by the dataset their `source.json` names: one
//! single-shot `content::diff` per pair, from the bytes in memory, as `benchmark_optimal_solutions`
//! times one `diff_code` per fixture. Unlike that figure, this one includes decoding the files
//! (unpacking, rasterising glyphs and pages), since for content that is most of the work.
//!
//! **Accuracy** is over the judged fixtures in `src/test/data/<family>/`: a fixture is perfect when
//! the engine's verdict equals every verdict the human gave on it (the pair's, and each judged
//! member's), and near-perfect when every one of the engine's levels is at most one level from the
//! human's, tags aside. Can't judge is left out; a fixture with nothing judged counts in neither.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;

use omnidiff::diff::content::{self, ContentDiff, Family, Verdict};
use omnidiff::test::helper::human_content::{self, Judgement};

#[derive(Parser)]
#[command(about = "Speed and accuracy of content diffs, per family")]
struct Args {
    /// Where the sampled pairs are.
    #[arg(long, default_value = "src/test/data/samples")]
    samples: PathBuf,
    /// Also write one row per timed pair to this CSV.
    #[arg(long)]
    csv: Option<PathBuf>,
}

#[derive(Default)]
struct Speed {
    milliseconds: Vec<f64>,
    /// Pairs whose bytes are not content OmniDiff diffs, or fail to decode: not timed.
    not_diffed: usize,
}

#[derive(Default)]
struct Accuracy {
    judged: usize,
    perfect: usize,
    near_perfect: usize,
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let rank = ((p / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

/// The family a sample was drawn for, from its `source.json`; `None` for code and text samples.
fn sample_family(dir: &Path) -> Option<Family> {
    let source = std::fs::read_to_string(dir.join("source.json")).ok()?;
    let source: serde_json::Value = serde_json::from_str(&source).ok()?;
    let dataset = source["dataset"].as_str()?;
    Family::ALL
        .into_iter()
        .find(|family| family.name() == dataset)
}

/// Every verdict the human gave on a fixture, beside the engine's (`None` where the engine sees no
/// change).
fn judged(
    human: &human_content::HumanContent,
    engine: &ContentDiff,
) -> Vec<(Verdict, Option<Verdict>)> {
    let mut pairs: Vec<(Verdict, Option<Verdict>)> = human
        .verdict
        .as_ref()
        .and_then(Judgement::verdict)
        .map(|verdict| (verdict, Some(engine.verdict())))
        .into_iter()
        .collect();
    let container = match engine {
        ContentDiff::Container(diff) => Some(diff),
        ContentDiff::Picture(_) => None,
    };
    pairs.extend(human.members.iter().filter_map(|(key, judgement)| {
        let found = container
            .and_then(|diff| diff.member(key))
            .and_then(|member| member.verdict());
        Some((judgement.verdict()?, found))
    }));
    pairs
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut speed: BTreeMap<Family, Speed> = BTreeMap::new();
    let mut rows = Vec::new();
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&args.samples)
        .with_context(|| format!("reading {:?}", args.samples))?
        .map(|entry| Ok(entry?.path()))
        .collect::<Result<_>>()?;
    dirs.sort();
    for dir in dirs {
        let Some(family) = sample_family(&dir) else {
            continue;
        };
        let (before, after) = human_content::pair_bytes_in(&dir)?;
        let started = Instant::now();
        let diff = content::diff(&before, &after);
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
        let entry = speed.entry(family).or_default();
        let outcome = match diff {
            Ok(Some(_)) => {
                entry.milliseconds.push(elapsed);
                "diffed"
            }
            Ok(None) => "not_content",
            Err(_) => "failed",
        };
        if outcome != "diffed" {
            entry.not_diffed += 1;
        }
        rows.push((family, dir, outcome, elapsed));
    }

    let mut accuracy: BTreeMap<Family, Accuracy> = BTreeMap::new();
    for family in Family::ALL {
        let entry = accuracy.entry(family).or_default();
        for dir in std::fs::read_dir(human_content::root(family))? {
            let dir = dir?.path();
            if !dir.is_dir() {
                continue;
            }
            let name = dir.file_name().unwrap_or_default().to_string_lossy();
            let human = human_content::load(family, &name)?;
            let engine = human_content::engine_diff(family, &name)?;
            let verdicts = judged(&human, &engine);
            if verdicts.is_empty() {
                continue;
            }
            entry.judged += 1;
            if verdicts
                .iter()
                .all(|(human, engine)| Some(*human) == *engine)
            {
                entry.perfect += 1;
            }
            if verdicts.iter().all(|(human, engine)| {
                engine.is_some_and(|engine| {
                    (human.level as usize).abs_diff(engine.level as usize) <= 1
                })
            }) {
                entry.near_perfect += 1;
            }
        }
    }

    println!(
        "| Family | Pairs timed | Not diffed | p50 ms | p90 ms | p99 ms | Slowest ms | Under 100ms | Judged | Perfect | Near-perfect |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|");
    let share = |part: usize, whole: usize| {
        if whole == 0 {
            "-".to_string()
        } else {
            format!("{:.1}%", 100.0 * part as f64 / whole as f64)
        }
    };
    for family in Family::ALL {
        let mut timed = speed.remove(&family).unwrap_or_default();
        timed.milliseconds.sort_by(f64::total_cmp);
        let times = &timed.milliseconds;
        let figure = |p: f64| {
            if times.is_empty() {
                "-".to_string()
            } else {
                format!("{:.1}", percentile(times, p))
            }
        };
        let fast = times.iter().filter(|ms| **ms < 100.0).count();
        let judged = accuracy.remove(&family).unwrap_or_default();
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            family.name(),
            times.len(),
            timed.not_diffed,
            figure(50.0),
            figure(90.0),
            figure(99.0),
            figure(100.0),
            share(fast, times.len()),
            judged.judged,
            share(judged.perfect, judged.judged),
            share(judged.near_perfect, judged.judged),
        );
    }

    if let Some(path) = args.csv {
        let mut writer = csv::Writer::from_path(&path)?;
        writer.write_record(["family", "sample", "outcome", "elapsed_ms"])?;
        for (family, dir, outcome, elapsed) in rows {
            let name = dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            writer.write_record([family.name(), &name, outcome, &format!("{elapsed:.2}")])?;
        }
        writer.flush()?;
    }
    Ok(())
}
