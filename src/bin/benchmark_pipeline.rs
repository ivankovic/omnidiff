/*  This file is part of the OmniDiff code diffing tool.
 *
 *  Copyright (C) 2026 Marko Ivankovic
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, version 3 of the License.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

//! The whole pipeline over the fixture corpus, stage by stage, for speed work that must not
//! change the output.
//!
//! `fingerprint` prints one line per fixture: hashes of the complete result - the node mapping,
//! the N:M groups, the embedded layers, and both presets' ranges, raw and filtered. Two builds
//! that print the same lines produce the same diffs. Node ids are addresses and differ between
//! runs, so every id is written as its preorder index.
//!
//! `time` runs the stages one after the other on one thread and prints what each cost, as
//! process CPU time (which counts the pipeline's own helper threads) and as wall-clock time.
//! `benchmark_optimal_solutions` times the mapping alone; this also times the parse, the metadata
//! and the range rendering. `--until` stops every fixture after a stage, so `perf stat` on two
//! runs gives one stage's instruction count.

use std::fmt::Write as _;
use std::hash::Hasher as _;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use omnidiff::code::{ASTMetadata, Code};
use omnidiff::diff::text::{RangeMatch, RenderOptions, TextDiff, ranges_for_options};
use omnidiff::diff::{ASTDiff, NodeCache};
use omnidiff::test::helper;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print each fixture's output hashes.
    Fingerprint {
        #[command(flatten)]
        selection: Selection,
        /// Fixtures hashed at once.
        #[arg(long, default_value_t = 1)]
        threads: usize,
        /// Also write each fixture's canonical text to `<DIR>/<fixture>.txt`.
        #[arg(long, value_name = "DIR")]
        dump: Option<std::path::PathBuf>,
    },
    /// Print what each stage costs.
    Time {
        #[command(flatten)]
        selection: Selection,
        /// Times each fixture is run; the totals are sums over every run.
        #[arg(long, default_value_t = 1)]
        repeat: usize,
        /// The last stage to run.
        #[arg(long, value_enum, default_value_t = Stage::Minimal)]
        until: Stage,
        /// Print one line per fixture as well as the totals.
        #[arg(long)]
        per_fixture: bool,
    },
}

#[derive(clap::Args)]
struct Selection {
    /// A file of fixture names, one per line; every fixture without it.
    #[arg(long, value_name = "FILE")]
    list: Option<std::path::PathBuf>,
    /// One fixture by name; may be repeated.
    #[arg(long, value_name = "NAME")]
    only: Vec<String>,
}

/// The pipeline's stages, in the order they run.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Stage {
    /// Reading both files and parsing them.
    Parse,
    /// The per-node metadata of both trees.
    Metadata,
    /// `diff_code`: the node mapping.
    Map,
    /// The ranges under `RenderOptions::FULL`: the node cache, `TextDiff` and the display filter.
    Full,
    /// The same under `RenderOptions::MINIMAL`.
    Minimal,
}

const STAGES: [Stage; 5] = [
    Stage::Parse,
    Stage::Metadata,
    Stage::Map,
    Stage::Full,
    Stage::Minimal,
];

fn selected(selection: &Selection) -> Result<Vec<(String, std::path::PathBuf)>> {
    let mut cases = helper::handmade_test_case_dirs()?;
    let mut names: Vec<String> = selection.only.clone();
    if let Some(list) = &selection.list {
        let text = std::fs::read_to_string(list).with_context(|| format!("reading {list:?}"))?;
        names.extend(
            text.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(String::from),
        );
    }
    if !names.is_empty() {
        let wanted: std::collections::HashSet<&str> = names.iter().map(String::as_str).collect();
        cases.retain(|(name, _)| wanted.contains(name.as_str()));
    }
    Ok(cases)
}

/// A node id as its preorder index plus one; `-` for the 0 of a delete or insert.
fn index_of(metadata: &ASTMetadata, id: usize) -> String {
    if id == 0 {
        return "-".to_string();
    }
    match metadata.node_info.get(&id) {
        Some(info) => (info.preorder_index + 1).to_string(),
        None => "?".to_string(),
    }
}

/// The mapping, the node maps, the groups in their own order and the embedded layers, each id as
/// a preorder index and every list but the groups sorted.
fn mapping_text(diff: &ASTDiff, before: &Code, after: &Code) -> (String, Vec<String>) {
    let before_metadata = omnidiff::code::metadata::metadata_of(before);
    let after_metadata = omnidiff::code::metadata::metadata_of(after);
    let b = |id: usize| index_of(&before_metadata, id);
    let a = |id: usize| index_of(&after_metadata, id);

    let mut lines: Vec<String> = diff
        .mapping
        .iter()
        .map(|((before_id, after_id), mapping)| {
            format!(
                "m {} {} {:?} {:?} {}",
                b(*before_id),
                a(*after_id),
                mapping.operation,
                mapping.reason,
                mapping.cost
            )
        })
        .collect();
    // The 0 key is a slot every delete or insert overwrites, so what it holds is not output.
    lines.extend(
        diff.before_node_map
            .iter()
            .filter(|(id, _)| **id != 0)
            .map(|(id, partner)| format!("b {} {}", b(*id), a(*partner))),
    );
    lines.extend(
        diff.after_node_map
            .iter()
            .filter(|(id, _)| **id != 0)
            .map(|(id, partner)| format!("a {} {}", a(*id), b(*partner))),
    );
    lines.sort_unstable();
    let mut text = lines.join("\n");

    let groups: Vec<String> = diff
        .groups
        .iter()
        .map(|group| {
            let before: Vec<String> = group.before.iter().map(|id| b(*id)).collect();
            let after: Vec<String> = group.after.iter().map(|id| a(*id)).collect();
            format!("g [{}] [{}]", before.join(","), after.join(","))
        })
        .collect();

    for embedded in &diff.embedded {
        let (inner, inner_groups) = mapping_text(&embedded.diff, &embedded.before, &embedded.after);
        write!(
            text,
            "\ne {} {} {:?} {:?} {:?} {:?} {:?} {:?}\n{}\n{}\nend",
            b(embedded.before_id),
            a(embedded.after_id),
            embedded.language,
            embedded.tokens,
            embedded.before.contents,
            embedded.after.contents,
            embedded.before_offsets,
            embedded.after_offsets,
            inner,
            inner_groups.join("\n"),
        )
        .expect("writing to a String");
    }
    (text, groups)
}

fn ranges_text(ranges: &[RangeMatch]) -> String {
    let mut text = String::new();
    for range in ranges {
        let (s, d) = (&range.source, &range.destination);
        writeln!(
            text,
            "{}:{}-{}:{} {}:{}-{}:{} {:?}",
            s.start_row,
            s.start_column,
            s.end_row,
            s.end_column,
            d.start_row,
            d.start_column,
            d.end_row,
            d.end_column,
            range.operation
        )
        .expect("writing to a String");
    }
    text
}

/// Both sides' ranges under `options`, as built and as the display filter leaves them; the
/// plain-text line diff for a pair without trees.
fn painted(before: &Code, after: &Code, diff: Option<&ASTDiff>, options: RenderOptions) -> String {
    let (before_ranges, after_ranges) = match diff {
        Some(diff) => {
            let node_cache = NodeCache::build(before, after);
            let text_diff = TextDiff::from_with_options(before, after, diff, &node_cache, options);
            (text_diff.all(0), text_diff.all(1))
        }
        None => omnidiff::diff::text::plain_text_line_diff(&before.contents, &after.contents),
    };
    let before_shown = ranges_for_options(&before_ranges, &before.contents, options);
    let after_shown = ranges_for_options(&after_ranges, &after.contents, options);
    format!(
        "before\n{}after\n{}before shown\n{}after shown\n{}",
        ranges_text(&before_ranges),
        ranges_text(&after_ranges),
        ranges_text(&before_shown),
        ranges_text(&after_shown),
    )
}

fn hash(text: &str) -> u64 {
    // `DefaultHasher::new` uses fixed keys, so the value is the same in every run.
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write(text.as_bytes());
    hasher.finish()
}

/// One fixture's line: the mapping, the groups sorted, the groups in their own order, and each
/// preset's ranges. `None` when the directory holds no pair.
fn fingerprint(
    name: &str,
    dir: &std::path::Path,
    dump: Option<&std::path::Path>,
) -> Result<Option<String>> {
    let Some((before, after)) = helper::code_pair_from_dir(dir)? else {
        return Ok(None);
    };
    let has_trees = before.ast.is_some() && after.ast.is_some();
    let diff = omnidiff::diff::diff_code(&before, &after);
    let ast_diff = diff.ast.as_ref().filter(|_| has_trees);

    let (mapping, groups) = match ast_diff {
        Some(ast_diff) => mapping_text(ast_diff, &before, &after),
        None => Default::default(),
    };
    let groups_in_order = groups.join("\n");
    let mut sorted = groups;
    sorted.sort_unstable();
    let groups_sorted = sorted.join("\n");
    let comment_only = ast_diff.is_some_and(|ast_diff| {
        let node_cache = NodeCache::build(&before, &after);
        omnidiff::diff::text::is_comment_only_diff(&before, &after, ast_diff, &node_cache)
    });
    let full = painted(&before, &after, ast_diff, RenderOptions::FULL);
    let minimal = painted(&before, &after, ast_diff, RenderOptions::MINIMAL);

    if let Some(dump) = dump {
        std::fs::write(
            dump.join(format!("{name}.txt")),
            format!(
                "{mapping}\n== groups\n{groups_in_order}\n== comment only {comment_only}\n\
                 == full\n{full}== minimal\n{minimal}"
            ),
        )?;
    }
    Ok(Some(format!(
        "{name} map={:016x} groups={:016x} group_order={:016x} full={:016x} minimal={:016x} \
         comment_only={comment_only}",
        hash(&mapping),
        hash(&groups_sorted),
        hash(&groups_in_order),
        hash(&full),
        hash(&minimal),
    )))
}

fn fingerprint_all(
    cases: &[(String, std::path::PathBuf)],
    threads: usize,
    dump: Option<&std::path::Path>,
) -> Result<()> {
    if let Some(dump) = dump {
        std::fs::create_dir_all(dump)?;
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let lines: Vec<std::sync::Mutex<Option<Result<Option<String>>>>> =
        cases.iter().map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            std::thread::Builder::new()
                .stack_size(omnidiff::tui::app::DIFF_COMPUTE_STACK_SIZE)
                .spawn_scoped(scope, || {
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some((name, dir)) = cases.get(i) else {
                            break;
                        };
                        let line = fingerprint(name, dir, dump);
                        *lines[i].lock().expect("no worker panics holding it") = Some(line);
                    }
                })
                .expect("spawn a fingerprint worker");
        }
    });
    for line in lines {
        if let Some(line) = line
            .into_inner()
            .expect("no worker panics holding it")
            .transpose()?
            .flatten()
        {
            println!("{line}");
        }
    }
    Ok(())
}

/// CPU time this process has used, over all its threads, in seconds.
#[cfg(unix)]
fn process_cpu_seconds() -> f64 {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `time` is a valid, writable `timespec`, which is all `clock_gettime` needs.
    let status = unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut time) };
    assert_eq!(status, 0, "clock_gettime(CLOCK_PROCESS_CPUTIME_ID) failed");
    time.tv_sec as f64 + time.tv_nsec as f64 * 1e-9
}

/// Without a process CPU clock the CPU column repeats the wall-clock one.
#[cfg(not(unix))]
fn process_cpu_seconds() -> f64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_secs_f64()
}

/// CPU and wall-clock milliseconds per stage.
#[derive(Default, Clone, Copy)]
struct Cost {
    cpu_ms: [f64; STAGES.len()],
    wall_ms: [f64; STAGES.len()],
}

impl Cost {
    fn add(&mut self, other: &Cost) {
        for i in 0..STAGES.len() {
            self.cpu_ms[i] += other.cpu_ms[i];
            self.wall_ms[i] += other.wall_ms[i];
        }
    }
}

struct Stopwatch {
    cpu: f64,
    wall: std::time::Instant,
}

impl Stopwatch {
    fn start() -> Self {
        Stopwatch {
            cpu: process_cpu_seconds(),
            wall: std::time::Instant::now(),
        }
    }

    /// Books the time since the last lap, or the start, to `stage`.
    fn lap(&mut self, stage: Stage, cost: &mut Cost) {
        let (cpu, wall) = (process_cpu_seconds(), std::time::Instant::now());
        cost.cpu_ms[stage as usize] += (cpu - self.cpu) * 1000.0;
        cost.wall_ms[stage as usize] += (wall - self.wall).as_secs_f64() * 1000.0;
        (self.cpu, self.wall) = (cpu, wall);
    }
}

/// Builds and filters both sides' ranges under `options`, as a viewer does before its first
/// frame, and returns how many ranges there are so the work cannot be optimised away.
fn render(before: &Code, after: &Code, diff: &ASTDiff, options: RenderOptions) -> usize {
    let node_cache = NodeCache::build(before, after);
    let text_diff = TextDiff::from_with_options(before, after, diff, &node_cache, options);
    ranges_for_options(&text_diff.all(0), &before.contents, options).len()
        + ranges_for_options(&text_diff.all(1), &after.contents, options).len()
}

/// One run of one fixture through every stage up to `until`. `None` when the directory holds no
/// pair.
fn time_one(dir: &std::path::Path, until: Stage) -> Result<Option<Cost>> {
    let mut cost = Cost::default();
    let mut watch = Stopwatch::start();
    let Some((mut before, mut after)) = helper::code_pair_from_dir_without_metadata(dir)? else {
        return Ok(None);
    };
    watch.lap(Stage::Parse, &mut cost);
    if until == Stage::Parse || before.ast.is_none() || after.ast.is_none() {
        return Ok(Some(cost));
    }

    before.ensure_parsed()?;
    after.ensure_parsed()?;
    watch.lap(Stage::Metadata, &mut cost);
    if until == Stage::Metadata {
        return Ok(Some(cost));
    }

    let diff = omnidiff::diff::diff_code(&before, &after);
    watch.lap(Stage::Map, &mut cost);
    let Some(ast_diff) = diff.ast.as_ref().filter(|_| until > Stage::Map) else {
        return Ok(Some(cost));
    };

    std::hint::black_box(render(&before, &after, ast_diff, RenderOptions::FULL));
    watch.lap(Stage::Full, &mut cost);
    if until == Stage::Full {
        return Ok(Some(cost));
    }

    std::hint::black_box(render(&before, &after, ast_diff, RenderOptions::MINIMAL));
    watch.lap(Stage::Minimal, &mut cost);
    Ok(Some(cost))
}

fn time_all(
    cases: &[(String, std::path::PathBuf)],
    repeat: usize,
    until: Stage,
    per_fixture: bool,
) -> Result<()> {
    let mut total = Cost::default();
    let mut fixtures = 0;
    for (name, dir) in cases {
        let mut fixture = Cost::default();
        let mut ran = false;
        for _ in 0..repeat.max(1) {
            if let Some(cost) = time_one(dir, until)? {
                fixture.add(&cost);
                ran = true;
            }
        }
        if !ran {
            continue;
        }
        fixtures += 1;
        total.add(&fixture);
        if per_fixture {
            let cells: Vec<String> = fixture.cpu_ms.iter().map(|ms| format!("{ms:.3}")).collect();
            println!("{name},{}", cells.join(","));
        }
    }
    println!(
        "{fixtures} fixtures x {} run(s); milliseconds, summed",
        repeat.max(1)
    );
    println!("{:<10} {:>12} {:>12}", "stage", "cpu", "wall");
    for stage in STAGES {
        println!(
            "{:<10} {:>12.1} {:>12.1}",
            format!("{stage:?}").to_lowercase(),
            total.cpu_ms[stage as usize],
            total.wall_ms[stage as usize]
        );
    }
    println!(
        "{:<10} {:>12.1} {:>12.1}",
        "total",
        total.cpu_ms.iter().sum::<f64>(),
        total.wall_ms.iter().sum::<f64>()
    );
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    // The diff recurses as deep as the tree; the TUI's diff thread has the same need.
    std::thread::Builder::new()
        .stack_size(omnidiff::tui::app::DIFF_COMPUTE_STACK_SIZE)
        .spawn(move || match args.command {
            Command::Fingerprint {
                selection,
                threads,
                dump,
            } => fingerprint_all(&selected(&selection)?, threads, dump.as_deref()),
            Command::Time {
                selection,
                repeat,
                until,
                per_fixture,
            } => time_all(&selected(&selection)?, repeat, until, per_fixture),
        })
        .expect("spawn the pipeline thread")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}
