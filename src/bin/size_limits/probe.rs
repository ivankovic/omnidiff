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

//! Runs one generated pair through the real `omnidiff` binary and judges the result.
//!
//! Everything that can fail runs in a child process: a grammar's C scanner can abort on its own
//! assertion, a deep tree can overflow a stack, a large one can exhaust memory, and none of that
//! may take the harness down with it. Each child gets a wall-clock budget and an address-space
//! cap (`RLIMIT_AS`), and no core file.
//!
//! Each child runs under this binary's `measure` subcommand ([`measure_command`]), which runs it
//! and reports its peak resident memory. Asking the kernel for a direct child's peak would not
//! do: Linux carries the parent's peak across `exec` into the child's `ru_maxrss`, so every
//! probe would report at least the harness's own memory, generated files and all. `measure` is
//! small when it starts the child. The child gets a process group of its own, so a timeout kills
//! `measure` and what it runs together.
//!
//! Two children per probe: `omnidiff --mode json` on the pair, and this binary's own `parse`
//! subcommand on the before side, which reports how the grammar did - node count, `ERROR` nodes,
//! whether the tree covers the file, and whether OmniDiff keeps it or diffs the file as text. The
//! JSON output does not say which of the two the diff took.
//!
//! This file is also compiled into `tests/size_limits.rs` with `#[path]`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use super::generate::{Axis, Template, changed_span, generate, position};

/// Resource limits for each child.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub timeout: Duration,
    /// Address-space cap in bytes; 0 for none.
    pub memory: u64,
}

/// How a child ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Exit {
    Code(i32),
    Signal(i32),
    TimedOut,
}

/// A finished child.
#[derive(Debug, Clone)]
pub struct Run {
    pub exit: Exit,
    pub stdout: String,
    pub stderr: String,
    pub elapsed: Duration,
    /// Peak resident set, in KiB.
    pub max_rss_kib: u64,
}

impl Run {
    /// True when the child ran out of the memory it was allowed, by any of the ways that shows.
    pub fn out_of_memory(&self) -> bool {
        const SIGNS: [&str; 6] = [
            "memory allocation of",
            "failed to allocate",
            // tree-sitter's own allocator: `tree-sitter failed to reallocate N bytes`.
            "failed to reallocate",
            "out of memory",
            "Cannot allocate memory",
            "failed to spawn thread",
        ];
        matches!(self.exit, Exit::Signal(_) | Exit::Code(_))
            && self.exit != Exit::Code(0)
            && SIGNS.iter().any(|sign| self.stderr.contains(sign))
    }

    /// The last non-empty line of stderr, for a one-line report.
    pub fn last_error_line(&self) -> String {
        let line = self
            .stderr
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty() && !line.starts_with("note: run with"))
            .unwrap_or("")
            .trim();
        let mut line: String = line.chars().take(160).collect();
        line = line.replace(',', ";");
        line
    }
}

/// Runs `program` with `args` under `limits`, through `measure` (the `size_limits` binary), its
/// output to files in `scratch` so a full pipe cannot stall it.
#[cfg(unix)]
pub fn run(
    measure: &Path,
    program: &Path,
    args: &[&std::ffi::OsStr],
    limits: Limits,
    scratch: &Path,
) -> std::io::Result<Run> {
    use std::os::unix::process::CommandExt;

    let stdout_path = scratch.join("stdout");
    let stderr_path = scratch.join("stderr");
    let rss_path = scratch.join("max_rss_kib");
    let _ = std::fs::remove_file(&rss_path);
    let mut command = Command::new(measure);
    command
        .arg("measure")
        .arg(&rss_path)
        .arg(program)
        .args(args);
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::fs::File::create(&stdout_path)?)
        .stderr(std::fs::File::create(&stderr_path)?)
        // Some failures panic; a backtrace would only lengthen stderr.
        .env("RUST_BACKTRACE", "0");
    let memory = limits.memory;
    // SAFETY: only async-signal-safe calls (setpgid, setrlimit) between fork and exec.
    unsafe {
        command.pre_exec(move || {
            libc::setpgid(0, 0);
            let no_core = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            libc::setrlimit(libc::RLIMIT_CORE, &no_core);
            if memory > 0 {
                let cap = libc::rlimit {
                    rlim_cur: memory as libc::rlim_t,
                    rlim_max: memory as libc::rlim_t,
                };
                if libc::setrlimit(libc::RLIMIT_AS, &cap) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }

    let started = Instant::now();
    let child = command.spawn()?;
    let pid = child.id() as libc::pid_t;
    // Reaped with waitpid below; `child` is never waited on, and dropping it does not.
    drop(child);

    let mut status: libc::c_int = 0;
    let mut timed_out = false;
    let mut pause = Duration::from_millis(2);
    loop {
        // SAFETY: `pid` is our own child, not yet reaped; the out-pointer is valid.
        let reaped = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if reaped == pid {
            break;
        }
        if reaped < 0 {
            return Err(std::io::Error::last_os_error());
        }
        if started.elapsed() >= limits.timeout {
            // SAFETY: as above. The group is `measure` and its child; a blocking waitpid after
            // SIGKILL reaps `measure`, and init the orphaned child.
            unsafe {
                libc::kill(-pid, libc::SIGKILL);
                libc::waitpid(pid, &mut status, 0);
            }
            timed_out = true;
            break;
        }
        std::thread::sleep(pause);
        pause = (pause * 2).min(Duration::from_millis(50));
    }
    let elapsed = started.elapsed();

    let exit = if timed_out {
        Exit::TimedOut
    } else if libc::WIFSIGNALED(status) {
        Exit::Signal(libc::WTERMSIG(status))
    } else {
        Exit::Code(libc::WEXITSTATUS(status))
    };
    let read = |path: &Path| {
        std::fs::read(path)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default()
    };
    Ok(Run {
        exit,
        stdout: read(&stdout_path),
        stderr: read(&stderr_path),
        elapsed,
        max_rss_kib: read(&rss_path).trim().parse().unwrap_or(0),
    })
}

/// The `measure` subcommand: runs `program`, writes its peak resident memory in KiB to
/// `rss_path`, and ends as it did - the same exit code, or the same signal raised on itself.
#[cfg(unix)]
pub fn measure_command(
    rss_path: &Path,
    program: &Path,
    args: &[std::ffi::OsString],
) -> anyhow::Result<()> {
    use std::os::unix::process::ExitStatusExt;

    let status = Command::new(program).args(args).status()?;
    // SAFETY: rusage is plain data, zero is a valid value; the out-pointer is valid.
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_CHILDREN, &mut usage) };
    // Linux reports ru_maxrss in KiB.
    std::fs::write(rss_path, usage.ru_maxrss.max(0).to_string())?;
    if let Some(signal) = status.signal() {
        // SAFETY: restoring the default action, then raising the signal on this process.
        unsafe {
            libc::signal(signal, libc::SIG_DFL);
            libc::raise(signal);
        }
    }
    std::process::exit(status.code().unwrap_or(1))
}

/// Only Unix has the `RLIMIT_AS` and `wait4` the probes rely on.
#[cfg(not(unix))]
pub fn run(
    _measure: &Path,
    _program: &Path,
    _args: &[&std::ffi::OsStr],
    _limits: Limits,
    _scratch: &Path,
) -> std::io::Result<Run> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "size_limits runs only on Unix",
    ))
}

#[cfg(not(unix))]
pub fn measure_command(
    _rss_path: &Path,
    _program: &Path,
    _args: &[std::ffi::OsString],
) -> anyhow::Result<()> {
    anyhow::bail!("size_limits runs only on Unix")
}

/// What the grammar made of a file, from the `parse` subcommand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parse {
    /// The parse finished: its node count, whether it holds `ERROR` or `MISSING` nodes, whether
    /// its root covers the file, and whether OmniDiff keeps the tree (otherwise: plain text).
    Tree {
        nodes: usize,
        errors: bool,
        covered: bool,
        kept: bool,
    },
    /// tree-sitter returned no tree.
    Failed,
    /// The child did not finish: crashed, timed out, or ran out of memory.
    Died(String),
}

impl Parse {
    pub fn summary(&self) -> String {
        match self {
            Parse::Tree {
                nodes,
                errors,
                covered,
                kept,
            } => format!(
                "{} nodes{}{}{}",
                nodes,
                if *errors { " ERROR" } else { "" },
                if *covered { "" } else { " UNCOVERED" },
                if *kept { "" } else { " text" },
            ),
            Parse::Failed => "no tree".into(),
            Parse::Died(how) => format!("parse {how}"),
        }
    }

    /// The `parse` subcommand's output line back to a [`Parse`].
    fn from_line(line: &str) -> Option<Parse> {
        let mut fields = std::collections::HashMap::new();
        for field in line.split_whitespace() {
            let (key, value) = field.split_once('=')?;
            fields.insert(key, value);
        }
        if fields.get("tree") == Some(&"none") {
            return Some(Parse::Failed);
        }
        Some(Parse::Tree {
            nodes: fields.get("nodes")?.parse().ok()?,
            errors: *fields.get("errors")? == "true",
            covered: *fields.get("covered")? == "true",
            kept: *fields.get("kept")? == "true",
        })
    }
}

/// The `parse` subcommand itself: parses `path` as OmniDiff does and prints one line.
pub fn parse_command(path: &Path) -> anyhow::Result<()> {
    let contents = std::fs::read_to_string(path)?;
    let language = omnidiff::code::language::language_for_path_and_content(path, &contents)
        .ok_or_else(|| anyhow::anyhow!("no language for {}", path.display()))?;
    let grammar = omnidiff::code::language::to_treesitter(&language)
        .ok_or_else(|| anyhow::anyhow!("no grammar for {language}"))?;
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&grammar)?;
    let Some(tree) = parser.parse(&contents, None) else {
        println!("tree=none");
        return Ok(());
    };
    let root = tree.root_node();
    let kept = omnidiff::code::language::parse_tree(&mut parser, &contents, language).is_some();
    println!(
        "nodes={} errors={} covered={} kept={kept}",
        root.descendant_count(),
        root.has_error(),
        omnidiff::code::language::covers_contents(&tree, &contents),
    );
    Ok(())
}

/// What a probe found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The change was reported on both sides, and nothing else was.
    Ok,
    /// The change was reported, along with hunks elsewhere: the diff is wrong but not silent.
    Noisy,
    /// The diff finished and did not report the change: a silent wrong answer.
    Missed,
    /// The diff exited non-zero or on a signal, for a reason other than memory.
    Crashed(String),
    TimedOut,
    OutOfMemory,
}

impl Outcome {
    pub fn name(&self) -> &'static str {
        match self {
            Outcome::Ok => "ok",
            Outcome::Noisy => "noisy",
            Outcome::Missed => "missed",
            Outcome::Crashed(_) => "crashed",
            Outcome::TimedOut => "timeout",
            Outcome::OutOfMemory => "oom",
        }
    }

    /// Whether growing further is pointless: the diff no longer finishes, or says nothing.
    pub fn is_failure(&self) -> bool {
        !matches!(self, Outcome::Ok | Outcome::Noisy)
    }
}

/// One probe's result.
#[derive(Debug, Clone)]
pub struct Probe {
    pub n: usize,
    pub lines: usize,
    pub bytes: usize,
    pub outcome: Outcome,
    pub parse: Parse,
    pub elapsed: Duration,
    pub max_rss_kib: u64,
    /// Hunks reported away from the change, both sides together.
    pub stray_hunks: usize,
}

/// A hunk from the JSON output: zero-based rows, byte columns.
#[derive(Debug, Clone, Copy)]
struct Hunk {
    start: (usize, usize),
    end: (usize, usize),
}

fn hunks(side: &serde_json::Value) -> Vec<Hunk> {
    side["hunks"]
        .as_array()
        .map(|hunks| {
            hunks
                .iter()
                .filter_map(|hunk| {
                    let range = &hunk["range"];
                    let at = |key: &str| range[key].as_u64().map(|v| v as usize);
                    Some(Hunk {
                        start: (at("start_row")?, at("start_column")?),
                        end: (at("end_row")?, at("end_column")?),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// How far, in bytes along one line, a hunk may sit from the change and still count as reporting
/// it: a token-level diff paints the whole edited token, and a line-level one the whole line.
const NEAR: usize = 64;

/// `(reported, stray)`: whether a hunk covers the change, and how many hunks are elsewhere.
fn judge(
    hunks: &[Hunk],
    change: ((usize, usize), (usize, usize)),
    long_line: bool,
) -> (bool, usize) {
    let ((start_row, start_col), (end_row, end_col)) = change;
    let mut reported = false;
    let mut stray = 0;
    for hunk in hunks {
        let rows_overlap = hunk.start.0 <= end_row && hunk.end.0 >= start_row;
        let near = rows_overlap
            && (!long_line
                || (hunk.start.1 <= end_col + NEAR && hunk.end.1 + NEAR >= start_col)
                || hunk.start.0 != hunk.end.0);
        if near {
            reported = true;
        } else {
            stray += 1;
        }
    }
    (reported, stray)
}

/// The paths a probe needs.
pub struct Tools<'a> {
    pub omnidiff: &'a Path,
    /// This binary, for its `parse` and `measure` subcommands.
    pub parser: &'a Path,
}

/// Generates the pair for `template`/`axis`/`n`, runs both children, and judges them.
pub fn probe(
    tools: &Tools,
    template: &Template,
    axis: Axis,
    n: usize,
    seed: u64,
    limits: Limits,
    scratch: &Path,
) -> anyhow::Result<Probe> {
    let before = generate(template, axis, n, seed, false);
    let after = generate(template, axis, n, seed, true);
    let dir: PathBuf = scratch.to_path_buf();
    let before_path = dir.join(format!("before.{}", template.extension));
    let after_path = dir.join(format!("after.{}", template.extension));
    std::fs::write(&before_path, &before)?;
    std::fs::write(&after_path, &after)?;

    let parse_run = run(
        tools.parser,
        tools.parser,
        &["parse".as_ref(), before_path.as_os_str()],
        limits,
        &dir,
    )?;
    let parse = if parse_run.exit == Exit::Code(0) {
        parse_run
            .stdout
            .lines()
            .find_map(Parse::from_line)
            .unwrap_or_else(|| Parse::Died("printed nothing".into()))
    } else {
        Parse::Died(describe(&parse_run))
    };

    let diff_run = run(
        tools.parser,
        tools.omnidiff,
        &[
            "--mode".as_ref(),
            "json".as_ref(),
            before_path.as_os_str(),
            after_path.as_os_str(),
        ],
        limits,
        &dir,
    )?;

    let (before_span, after_span) = changed_span(&before, &after);
    let long_line = matches!(axis, Axis::Width | Axis::Token);
    let mut stray_hunks = 0;
    let outcome = match &diff_run.exit {
        Exit::TimedOut => Outcome::TimedOut,
        _ if diff_run.out_of_memory() => Outcome::OutOfMemory,
        Exit::Code(0) => match serde_json::from_str::<serde_json::Value>(&diff_run.stdout) {
            Ok(json) => {
                let side = |text: &str, span: (usize, usize)| {
                    (position(text, span.0), position(text, span.1.max(span.0)))
                };
                let (before_reported, before_stray) = judge(
                    &hunks(&json["before"]),
                    side(&before, before_span),
                    long_line,
                );
                let (after_reported, after_stray) =
                    judge(&hunks(&json["after"]), side(&after, after_span), long_line);
                stray_hunks = before_stray + after_stray;
                if !before_reported || !after_reported {
                    Outcome::Missed
                } else if stray_hunks > 0 {
                    Outcome::Noisy
                } else {
                    Outcome::Ok
                }
            }
            Err(error) => Outcome::Crashed(format!("unreadable JSON: {error}")),
        },
        _ => Outcome::Crashed(describe(&diff_run)),
    };

    Ok(Probe {
        n,
        lines: before.lines().count(),
        bytes: before.len(),
        outcome,
        parse,
        elapsed: diff_run.elapsed,
        max_rss_kib: diff_run.max_rss_kib,
        stray_hunks,
    })
}

/// One line on how a child that did not succeed ended.
fn describe(run: &Run) -> String {
    let how = match &run.exit {
        Exit::TimedOut => return "timeout".into(),
        _ if run.out_of_memory() => return "oom".into(),
        Exit::Code(code) => format!("exit {code}"),
        Exit::Signal(signal) => format!("signal {signal}"),
    };
    format!("{how}: {}", run.last_error_line())
}
