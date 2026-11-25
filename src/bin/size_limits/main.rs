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

//! Finds how large an input OmniDiff handles, per language and per [`Axis`]: how many lines, how
//! deep a nesting, how long a line, how long a token.
//!
//! `size_limits search` grows a generated pair (`generate.rs`) along each axis, doubling `n` until
//! the diff fails or the file passes `--max-bytes`, then bisects between the last size that passed
//! and the first that failed. A failure is a diff that crashes, runs out of time or memory, or
//! finishes without reporting the one change the pair holds (`probe::Outcome`). Each probe runs in
//! child processes (`probe.rs`), so a grammar that aborts the process is a row in the report, not
//! the end of the run. Bisection assumes failure is monotonic in `n`; where it is not, the limit
//! it finds is one failing size, not the smallest.
//!
//! `size_limits check` runs one fixed size per axis that every language must pass
//! ([`CHECK_SIZES`]), plus a small size that must parse without errors (which checks the
//! generators themselves); `tests/size_limits.rs` runs the same checks. `size_limits parse FILE`
//! is the child the probes use to see what the grammar made of a file, and `size_limits measure`
//! the wrapper every child runs under (`probe.rs`).

mod check;
mod generate;
mod probe;

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

use check::{CHECK_SIZES, check_template};
use generate::{Axis, Template, templates};
use probe::{Limits, Outcome, Parse, Probe, Tools, probe};

#[derive(Parser, Debug)]
#[command(about = "Find the input sizes OmniDiff handles, per language and dimension")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    common: Common,
}

#[derive(clap::Args, Debug, Clone)]
struct Common {
    /// Only these languages (stable names, e.g. `YAML`, `CPP`, `ShellScript`); repeatable.
    #[arg(long = "language", global = true)]
    languages: Vec<String>,
    /// Only these axes: lines, depth, width, token; repeatable.
    #[arg(long = "axis", global = true)]
    axes: Vec<String>,
    /// Wall-clock budget per child, in seconds.
    #[arg(long, default_value_t = 60, global = true)]
    timeout: u64,
    /// Address-space cap per child, in GiB (0 for none).
    #[arg(long, default_value_t = 4, global = true)]
    memory_gib: u64,
    /// Probes run at once. Each may use up to `--memory-gib`.
    #[arg(long, default_value_t = 2, global = true)]
    jobs: usize,
    /// Seed for the generated files.
    #[arg(long, default_value_t = 1, global = true)]
    seed: u64,
    /// The omnidiff binary; defaults to the one next to this binary.
    #[arg(long, global = true)]
    omnidiff: Option<PathBuf>,
    /// Write every probe to this CSV file.
    #[arg(long, global = true)]
    csv: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Grow each pair until it fails, then bisect (the default).
    Search {
        /// Stop growing once a side would be larger than this, in MiB.
        #[arg(long, default_value_t = 64)]
        max_mib: usize,
        /// Bisection probes after the first failure.
        #[arg(long, default_value_t = 6)]
        refine: usize,
    },
    /// Every language must pass `CHECK_SIZES`; exits 1 otherwise.
    Check,
    /// Parse FILE as OmniDiff does and print what the grammar made of it (the probes' child).
    #[command(hide = true)]
    Parse { file: PathBuf },
    /// Run PROGRAM and write its peak resident memory to RSS_FILE (wraps every probe's child).
    #[command(hide = true)]
    Measure {
        rss_file: PathBuf,
        program: PathBuf,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<std::ffi::OsString>,
    },
}

/// One `(language, axis)` search's probes, in the order they ran.
struct Search {
    template: usize,
    axis: Axis,
    probes: Vec<Probe>,
}

fn start_size(axis: Axis) -> usize {
    match axis {
        Axis::Lines | Axis::Width | Axis::Token => 256,
        Axis::Depth => 8,
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command.unwrap_or(Command::Search {
        max_mib: 64,
        refine: 6,
    }) {
        Command::Parse { file } => probe::parse_command(&file),
        Command::Measure {
            rss_file,
            program,
            args,
        } => probe::measure_command(&rss_file, &program, &args),
        Command::Search { max_mib, refine } => search_all(&args.common, max_mib << 20, refine),
        Command::Check => {
            if check_all(&args.common)? {
                Ok(())
            } else {
                std::process::exit(1)
            }
        }
    }
}

/// The templates and axes the flags select.
fn selection(common: &Common) -> Result<(Vec<Template>, Vec<Axis>)> {
    let mut chosen = templates();
    if !common.languages.is_empty() {
        let names: Vec<String> = common.languages.iter().map(|l| l.to_lowercase()).collect();
        chosen.retain(|t| {
            let stable = omnidiff::code::language::stable_name(t.language).to_lowercase();
            names.contains(&stable) || names.contains(&t.extension.to_string())
        });
        if chosen.is_empty() {
            bail!("no language matches {:?}", common.languages);
        }
    }
    let axes = if common.axes.is_empty() {
        Axis::ALL.to_vec()
    } else {
        common
            .axes
            .iter()
            .map(|name| Axis::from_name(name).with_context(|| format!("unknown axis {name}")))
            .collect::<Result<_>>()?
    };
    Ok((chosen, axes))
}

fn tools_paths(common: &Common) -> Result<(PathBuf, PathBuf)> {
    let me = std::env::current_exe()?;
    let omnidiff = match &common.omnidiff {
        Some(path) => path.clone(),
        None => me.with_file_name(format!("omnidiff{}", std::env::consts::EXE_SUFFIX)),
    };
    if !omnidiff.exists() {
        bail!(
            "{} not found: build it (cargo build --release) or pass --omnidiff",
            omnidiff.display()
        );
    }
    Ok((omnidiff, me))
}

fn limits(common: &Common) -> Limits {
    Limits {
        timeout: Duration::from_secs(common.timeout),
        memory: common.memory_gib << 30,
    }
}

/// Runs `work` over `jobs` on `common.jobs` threads, each with its own scratch directory.
fn parallel<J: Send, R: Send>(
    common: &Common,
    jobs: Vec<J>,
    work: impl Fn(J, &Path) -> R + Sync,
) -> Result<Vec<R>> {
    let queue = Mutex::new(jobs.into_iter().enumerate().collect::<Vec<_>>());
    let results = Mutex::new(Vec::new());
    std::thread::scope(|scope| -> Result<()> {
        let mut handles = Vec::new();
        for _ in 0..common.jobs.max(1) {
            handles.push(scope.spawn(|| -> Result<()> {
                let scratch = tempfile::tempdir()?;
                loop {
                    let job = queue.lock().unwrap().pop();
                    let Some((index, job)) = job else {
                        return Ok(());
                    };
                    let result = work(job, scratch.path());
                    results.lock().unwrap().push((index, result));
                }
            }));
        }
        for handle in handles {
            handle.join().expect("a worker panicked")?;
        }
        Ok(())
    })?;
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|(index, _)| *index);
    Ok(results.into_iter().map(|(_, result)| result).collect())
}

fn csv_header() -> &'static str {
    "language,axis,n,lines,bytes,outcome,detail,parse,elapsed_ms,max_rss_mib,stray_hunks"
}

fn csv_row(template: &Template, axis: Axis, p: &Probe) -> String {
    let detail = match &p.outcome {
        Outcome::Crashed(how) => how.replace(',', ";"),
        _ => String::new(),
    };
    format!(
        "{},{},{},{},{},{},{},{},{},{},{}",
        omnidiff::code::language::stable_name(template.language),
        axis.name(),
        p.n,
        p.lines,
        p.bytes,
        p.outcome.name(),
        detail,
        p.parse.summary().replace(',', ";"),
        p.elapsed.as_millis(),
        p.max_rss_kib / 1024,
        p.stray_hunks,
    )
}

fn log_probe(template: &Template, axis: Axis, p: &Probe) {
    eprintln!(
        "{:>10} {:<5} n={:<9} {:>9} lines {:>10} bytes  {:<8} {:>7} ms {:>6} MiB  {}{}",
        omnidiff::code::language::stable_name(template.language),
        axis.name(),
        p.n,
        p.lines,
        p.bytes,
        p.outcome.name(),
        p.elapsed.as_millis(),
        p.max_rss_kib / 1024,
        p.parse.summary(),
        match &p.outcome {
            Outcome::Crashed(how) => format!("  [{how}]"),
            _ => String::new(),
        },
    );
}

fn search_all(common: &Common, max_bytes: usize, refine: usize) -> Result<()> {
    let (chosen, axes) = selection(common)?;
    let (omnidiff, me) = tools_paths(common)?;
    let tools = Tools {
        omnidiff: &omnidiff,
        parser: &me,
    };
    let limits = limits(common);
    let jobs: Vec<(usize, Axis)> = (0..chosen.len())
        .flat_map(|t| axes.iter().map(move |&axis| (t, axis)))
        .collect();
    let searches = parallel(common, jobs, |(t, axis), scratch| -> Result<Search> {
        let template = &chosen[t];
        let mut probes: Vec<Probe> = Vec::new();
        let run = |n: usize, probes: &mut Vec<Probe>| -> Result<Probe> {
            let p = probe(&tools, template, axis, n, common.seed, limits, scratch)?;
            log_probe(template, axis, &p);
            probes.push(p.clone());
            Ok(p)
        };
        // Grow.
        let mut n = start_size(axis);
        let mut last_ok: Option<usize> = None;
        let mut first_fail: Option<usize> = None;
        loop {
            let p = run(n, &mut probes)?;
            if p.outcome.is_failure() {
                first_fail = Some(n);
                break;
            }
            last_ok = Some(n);
            if p.bytes.saturating_mul(2) > max_bytes {
                break;
            }
            n *= 2;
        }
        // Bisect.
        if let (Some(mut lo), Some(mut hi)) = (last_ok, first_fail) {
            for _ in 0..refine {
                if hi - lo <= (lo / 32).max(1) {
                    break;
                }
                let mid = lo + (hi - lo) / 2;
                if run(mid, &mut probes)?.outcome.is_failure() {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
        }
        Ok(Search {
            template: t,
            axis,
            probes,
        })
    })?;
    let searches: Vec<Search> = searches.into_iter().collect::<Result<_>>()?;

    if let Some(path) = &common.csv {
        let mut out = std::fs::File::create(path)?;
        writeln!(out, "{}", csv_header())?;
        for search in &searches {
            let mut probes = search.probes.clone();
            probes.sort_by_key(|p| p.n);
            for p in &probes {
                writeln!(out, "{}", csv_row(&chosen[search.template], search.axis, p))?;
            }
        }
    }
    print_summary(&chosen, &searches);
    Ok(())
}

/// One line per `(language, axis)`: the largest size that passed, the smallest that failed and
/// how, and where the tree stopped being structural or clean.
fn print_summary(chosen: &[Template], searches: &[Search]) {
    println!(
        "{:<11} {:<5}  {:<34}  {:<44}  {:<12}  {:<12}  {:<12}",
        "language",
        "axis",
        "largest passing",
        "smallest failing",
        "ERROR from",
        "text from",
        "noisy from"
    );
    let mut by_key: BTreeMap<(String, Axis), &Search> = BTreeMap::new();
    for search in searches {
        let name = omnidiff::code::language::stable_name(chosen[search.template].language);
        by_key.insert((name.to_string(), search.axis), search);
    }
    for ((name, axis), search) in by_key {
        let mut probes = search.probes.clone();
        probes.sort_by_key(|p| p.n);
        let passing = probes.iter().rfind(|p| !p.outcome.is_failure());
        let failing = probes.iter().find(|p| p.outcome.is_failure());
        let first = |pred: &dyn Fn(&Probe) -> bool| {
            probes
                .iter()
                .find(|p| pred(p))
                .map_or("-".to_string(), |p| p.n.to_string())
        };
        let errors_from = first(&|p| matches!(p.parse, Parse::Tree { errors: true, .. }));
        let text_from = first(&|p| !matches!(p.parse, Parse::Tree { kept: true, .. }));
        let noisy_from = first(&|p| p.outcome == Outcome::Noisy);
        let passing = passing.map_or("-".to_string(), |p| {
            format!(
                "n={} ({} B, {} ms, {} MiB)",
                p.n,
                p.bytes,
                p.elapsed.as_millis(),
                p.max_rss_kib / 1024
            )
        });
        let failing = failing.map_or("none below the cap".to_string(), |p| {
            let mut how = match &p.outcome {
                Outcome::Crashed(how) => format!("crashed: {how}"),
                other => other.name().to_string(),
            };
            how.truncate(36);
            format!("n={} {how}", p.n)
        });
        println!(
            "{name:<11} {:<5}  {passing:<34}  {failing:<44}  {errors_from:<12}  {text_from:<12}  {noisy_from:<12}",
            axis.name()
        );
    }
}

fn check_all(common: &Common) -> Result<bool> {
    let (chosen, axes) = selection(common)?;
    let (omnidiff, me) = tools_paths(common)?;
    let tools = Tools {
        omnidiff: &omnidiff,
        parser: &me,
    };
    let limits = limits(common);
    let results = parallel(common, (0..chosen.len()).collect(), |t, scratch| {
        check_template(&tools, &chosen[t], &axes, common.seed, limits, scratch)
    })?;
    let mut clean = true;
    for findings in results {
        for finding in findings? {
            clean = false;
            println!(
                "FAIL {} {} n={}: {}",
                finding.language,
                finding.axis.name(),
                finding.n,
                finding.problem
            );
        }
    }
    if clean {
        println!("every language passed {CHECK_SIZES:?}");
    }
    Ok(clean)
}
