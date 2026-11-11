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
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::Context;
use anyhow::Result;
use clap::{Parser, Subcommand};

use omnidiff::tui;
use omnidiff::tui::headless::{Pair, PairContents, printable, printable_path};
use omnidiff::tui::osc1717;
use omnidiff::tui::positional::{
    binary_notice, content_notice, display_names, invoked_as_git_external_diff, record_path,
    resolve_before_after,
};

mod configure_prompt;
mod git_configure;
mod jj_configure;

/// The static musl builds only: musl's own malloc is markedly slower on this allocation-heavy
/// tool, and mimalloc closes the gap (see Cargo.toml's musl dependency).
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Top-level subcommands, coexisting with `Args::paths`: each subcommand name becomes a reserved
/// word for the first positional path, so a file literally named `git` cannot be diffed directly.
#[derive(Subcommand)]
enum Command {
    /// git integration helpers.
    Git {
        #[command(subcommand)]
        action: GitAction,
    },
    /// Jujutsu (jj) integration helpers.
    Jj {
        #[command(subcommand)]
        action: JjAction,
    },
    /// Generate packaging artifacts (shell completions, man page).
    // Nested under `util` so only one implausible filename is reserved, not `completions` and `man`.
    Util {
        #[command(subcommand)]
        action: UtilAction,
    },
}

#[derive(Subcommand)]
enum UtilAction {
    /// Print a shell completion script for SHELL on stdout.
    ///
    /// For example: `source <(omnidiff util completions bash)`.
    Completions {
        /// The shell to generate for.
        shell: clap_complete::Shell,
    },
    /// Print a roff-formatted man page (section 1) on stdout.
    Man,
    /// Print how the picture view would draw in this terminal - `kitty`, `sixel`, `iterm2` or
    /// `halfblocks` - and why. Asks the terminal as the TUI does, so run it where the TUI would
    /// run. `OMNIDIFF_GRAPHICS` overrides the detection.
    Graphics,
}

#[derive(Subcommand)]
enum GitAction {
    /// Interactively configure omnidiff as git's diff tool.
    Configure,
}

#[derive(Subcommand)]
enum JjAction {
    /// Interactively configure omnidiff as jj's diff tool.
    Configure,
}

/// When headless output should be ANSI-colored; see `use_color`.
#[derive(clap::ValueEnum, Clone, Copy, PartialEq, Eq, Debug, Default)]
enum ColorChoice {
    #[default]
    Auto,
    Always,
    Never,
}

/// How the diff is shown; see `should_run_headless` and `should_run_json`.
#[derive(clap::ValueEnum, Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Mode {
    /// The interactive viewer.
    #[default]
    Tui,
    /// Plain text on stdout.
    Headless,
    /// One JSON object on stdout, for editor integrations.
    Json,
}

/// The exit status of a failed run: what `--help` documents, and distinct from `--exit-code`'s 1.
const EXIT_FAILURE: i32 = 2;

/// A rate for the TUI's tick and render intervals: `Duration::from_secs_f64(1.0 / rate)` panics
/// on zero, a negative or NaN, so those are rejected as arguments.
fn parse_positive_rate(value: &str) -> Result<f64, String> {
    let rate: f64 = value.parse().map_err(|e| format!("{e}"))?;
    if rate.is_finite() && rate > 0.0 {
        Ok(rate)
    } else {
        Err("must be a positive number".to_string())
    }
}

#[derive(Parser)]
#[command(
    // Explicit: without it clap's derive takes the nearest preceding doc comment (`Command`'s)
    // as the description in `--help` and the man page.
    about = "Fast, robust, accurate content-aware diffing",
    long_about = "Fast, robust, accurate content-aware diffing: code by its syntax, and \
        pictures, archives, fonts, cursors, message catalogs and PDFs by what they hold.\n\n\
        With no arguments, opens an interactive two-panel terminal UI. With BEFORE and AFTER \
        file paths, diffs them directly - as the TUI, as plain text (--headless), or as a single \
        JSON object (--mode json) for editor integrations. Also serves as a `git difftool` \
        backend and a `jj` diff formatter; see `omnidiff git configure` and `omnidiff jj \
        configure`.",
    version,
    after_help = "Exit codes: 0 on success, 2 on error. Pass --exit-code to additionally get \
    1 when the files differ (the diff(1) convention), which is off by default for the same \
    reason `git diff` defaults to 0: when a VCS drives omnidiff as a display tool, a non-zero \
    exit means \"the tool failed\", not \"the files differ\"."
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// The files to diff: `BEFORE AFTER`, or git's 7- or 9-argument `GIT_EXTERNAL_DIFF` list
    /// (`path old-file old-hex old-mode new-file new-hex new-mode [new-path score]`). Without them
    /// the viewer starts empty.
    paths: Vec<PathBuf>,

    /// How to show the diff, `tui` by default. `headless` and `json` need BEFORE and AFTER.
    /// `headless` is also chosen when stdout is not a terminal, and under git's
    /// `GIT_EXTERNAL_DIFF` argument form unless `--mode tui` is given; `json` never is.
    // An `Option` so that an explicit `--mode tui` can be told from the default.
    #[arg(long, value_enum, ignore_case = true)]
    mode: Option<Mode>,

    /// Shorthand for `--mode headless`. `--batch` is a synonym.
    #[arg(long, alias = "batch", conflicts_with = "mode")]
    headless: bool,

    /// Paint only the ranges that carry meaning: every option of the TUI's `M` panel off (its
    /// Minimal preset), so standalone brackets and separators, leading whitespace, and moves that
    /// only reindent or resize go unpainted. Without this or `--full`, the panel's last saved
    /// setting applies; either one applies to this run only and is not saved.
    ///
    /// `--minimal` and `--full` are two faithful readings of the same diff, not a right and a
    /// wrong one.
    #[arg(long, conflicts_with = "full")]
    minimal: bool,

    /// Paint the fullest reading: every option of the `M` panel on except whole-pair updates
    /// (its Full preset).
    #[arg(long)]
    full: bool,

    /// Highlight an updated node's matched pair whole (e.g. both `argument` and
    /// `i_am_an_argument`), instead of only the part that differs.
    ///
    /// Independent of `--minimal`/`--full`; combine freely.
    #[arg(long)]
    whole_updates: bool,

    /// Paint a node as moved even when it only changed indentation (nesting added or removed
    /// around untouched content) - the `M` panel's "Paint reindent-only moves" row, forced on.
    ///
    /// Combine freely with the other render flags; it can only turn the option on.
    #[arg(long)]
    paint_reindent_moves: bool,

    /// When to ANSI-color headless output. `auto` colors unless `NO_COLOR` is set, even when
    /// stdout is a pipe (git's pager shows color); `always` and `never` override `NO_COLOR`.
    #[arg(long, value_enum, value_name = "WHEN", default_value_t = ColorChoice::Auto)]
    color: ColorChoice,

    /// Exit 1 when the files differ (0 when identical, 2 on error) - the `diff(1)` convention,
    /// for scripts and CI conditionals. Headless and JSON modes only.
    ///
    /// Off by default because version control systems read any non-zero exit from a display
    /// tool as a failure: jj warns on every file, and `git difftool` with
    /// `difftool.trustExitCode=true` aborts the whole diff. Ignored under git's
    /// `GIT_EXTERNAL_DIFF` argument form for the same reason.
    #[arg(long)]
    exit_code: bool,

    /// Unchanged lines to keep around each change in headless output, like `diff -U N`.
    #[arg(long, value_name = "N", default_value_t = tui::headless::CONTEXT_LINES)]
    context: usize,

    /// Open on the git review picker - the repository around the current directory's unstaged
    /// files, staged files and recent commits - instead of an empty viewer (the `G` key, at
    /// startup). TUI only; takes no BEFORE/AFTER pair.
    #[arg(long, conflicts_with_all = ["paths", "headless", "mode"])]
    review: bool,

    /// The TUI's tick rate, in ticks per second.
    // Hidden: a tuning knob for development, not a user-facing option.
    #[arg(long, hide = true, value_name = "FLOAT", default_value_t = 4.0, value_parser = parse_positive_rate)]
    tui_tick_rate: f64,

    /// The TUI's frame rate, in frames per second.
    #[arg(long, hide = true, value_name = "FLOAT", default_value_t = 60.0, value_parser = parse_positive_rate)]
    tui_frame_rate: f64,
}

async fn tui_main(args: &Args, before_after: Option<(PathBuf, PathBuf)>) -> Result<()> {
    tui::initialize_logging()?;
    tui::ui::install_panic_hook();

    let mut app = tui::app::App::new(args.tui_tick_rate, args.tui_frame_rate)?;
    if let Some(options) = render_option_flags(args) {
        app.override_render_options(options);
    }
    if let Some((before, after)) = before_after {
        app.open_files(before, after)?;
    }
    if args.review {
        app.start_in_review();
    }
    app.run().await?;

    Ok(())
}

/// The mode asked for, [`Mode::Tui`] when none was.
fn mode(args: &Args) -> Mode {
    args.mode.unwrap_or_default()
}

/// Whether to print text instead of starting the TUI: when asked to, whenever stdout is not a
/// terminal, which is what makes `GIT_EXTERNAL_DIFF` under git's pager work with no configuration,
/// and under git's 7- or 9-argument `GIT_EXTERNAL_DIFF` form even on a terminal - unless
/// `--mode tui` is given explicitly. git waits for an external diff to print its file and go on
/// to the next one: `git --no-pager diff` and lazygit (which renders an external diff on a
/// pseudo-terminal) would otherwise block on a full-screen viewer. `git difftool`'s two
/// arguments keep the viewer.
fn should_run_headless(args: &Args, stdout_is_terminal: bool) -> bool {
    args.headless
        || mode(args) == Mode::Headless
        || !stdout_is_terminal
        || (invoked_as_git_external_diff(&args.paths) && args.mode != Some(Mode::Tui))
}

/// Whether to print a JSON diff object. Only `--mode json` selects it: a non-terminal stdout must
/// keep getting headless text, or every pipe and git integration would change format.
fn should_run_json(args: &Args) -> bool {
    mode(args) == Mode::Json
}

/// The error for a text-mode run with no files, naming the reason text mode was chosen: a user
/// who passed `--headless` on a terminal is told about the missing files, not about the terminal.
fn headless_needs_files_message(args: &Args) -> &'static str {
    if args.headless || mode(args) == Mode::Headless {
        "text mode needs BEFORE and AFTER - pass two files to diff"
    } else {
        "stdout is not a terminal and no files were given - pass BEFORE and AFTER to run in text \
         mode, or run from a real terminal to use the interactive viewer"
    }
}

/// The render options the command line asks for, or `None` when it names none, so the `M`
/// panel's saved setting applies. A preset flag replaces that setting outright (a script's output
/// then does not depend on the machine); the single-option flags only ever turn an option on, on
/// top of whichever applies.
fn render_option_flags(args: &Args) -> Option<omnidiff::diff::text::RenderOptions> {
    use omnidiff::diff::text::RenderOptions;
    let preset = if args.minimal {
        Some(RenderOptions::MINIMAL)
    } else if args.full {
        Some(RenderOptions::FULL)
    } else {
        None
    };
    if !args.whole_updates && !args.paint_reindent_moves {
        return preset;
    }
    let mut options = preset.unwrap_or_else(omnidiff::tui::theme::load_render_options);
    options.whole_pair_updates |= args.whole_updates;
    options.paint_reindent_only_moves |= args.paint_reindent_moves;
    Some(options)
}

/// The render options a headless or JSON run paints with.
fn render_options(args: &Args) -> omnidiff::diff::text::RenderOptions {
    render_option_flags(args).unwrap_or_else(omnidiff::tui::theme::load_render_options)
}

/// Whether headless output should be ANSI-colored. `Auto` deliberately ignores whether stdout is a
/// terminal: headless mode's main caller is git's pager, a pipe that shows color; `NO_COLOR`
/// (<https://no-color.org>) is the opt-out.
fn use_color(choice: ColorChoice) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => !no_color_is_set(std::env::var_os("NO_COLOR")),
    }
}

/// no-color.org: the variable opts out when present *and non-empty*; `NO_COLOR=` is not an opt-out.
fn no_color_is_set(value: Option<std::ffi::OsString>) -> bool {
    value.is_some_and(|value| !value.is_empty())
}

/// The exit code of a completed non-interactive run (errors exit 2 at the call sites). The
/// `GIT_EXTERNAL_DIFF` form always exits 0: git reads non-zero there as "external diff died" and
/// aborts the entire multi-file diff.
fn exit_code_for(differed: bool, want_exit_code: bool, invoked_as_git_external_diff: bool) -> i32 {
    if differed && want_exit_code && !invoked_as_git_external_diff {
        1
    } else {
        0
    }
}

/// Reports a pair with at least one binary side (the other may be git's empty `/dev/null`), in
/// every mode. It must succeed: a UTF-8 decode error would exit 2, and under `GIT_EXTERNAL_DIFF`
/// git then abandons every remaining file in the diff. `record_file` is the OSC 1717 `file` field
/// when records were negotiated: every line of the notice is then tagged `f`.
fn run_binary(
    args: &Args,
    before: &Path,
    after: &Path,
    bytes: (&[u8], &[u8]),
    record_file: Option<&str>,
) -> Result<i32> {
    let (before_bytes, after_bytes) = bytes;
    let differed = before_bytes != after_bytes;
    // Content that does not decode is reported as any other binary, never as an error.
    let content = omnidiff::diff::content::diff(before_bytes, after_bytes)
        .ok()
        .flatten();
    if should_run_json(args) {
        // Still a JSON object of the usual shape (flagged `binary`, no hunks): prose would break
        // every `--mode json` consumer.
        let mut json = tui::json_output::binary_diff_json(before, after, content.as_ref())?;
        json.push('\n');
        tui::headless::write_stdout(&json)?;
    } else {
        let notice = match &content {
            Some(content) => content_notice(&args.paths, before, after, content),
            None => binary_notice(&args.paths, before, after, differed),
        };
        let notice = match record_file {
            Some(file) => osc1717::tag_lines(&notice, &osc1717::file_record(file)),
            None => notice,
        };
        tui::headless::write_stdout(&notice)?;
    }
    Ok(exit_code_for(
        differed,
        args.exit_code,
        invoked_as_git_external_diff(&args.paths),
    ))
}

/// A headless or JSON run on one pair: each side read once ([`Pair`]), then reported as binary or
/// diffed as text, with OSC 1717 records naming `record_file` if given.
fn run_non_interactive(
    args: &Args,
    before: &Path,
    after: &Path,
    record_file: Option<&str>,
) -> Result<i32> {
    let pair = Pair::read(before, after)?;
    if let PairContents::Binary {
        before: before_bytes,
        after: after_bytes,
    } = &pair.contents
    {
        return run_binary(
            args,
            before,
            after,
            (before_bytes, after_bytes),
            record_file,
        );
    }
    let differed = if should_run_json(args) {
        tui::json_output::run(pair, render_options(args))?
    } else {
        tui::headless::run(
            pair,
            use_color(args.color),
            args.context,
            render_options(args),
            display_names(&args.paths),
            record_file.map(str::to_string),
        )?
    };
    Ok(exit_code_for(
        differed,
        args.exit_code,
        invoked_as_git_external_diff(&args.paths),
    ))
}

/// [`run_non_interactive`], with a panic in the decoders or the diff - third-party code on
/// attacker-chosen bytes - contained (`tui::panic_guard`). Under `GIT_EXTERNAL_DIFF` a failure of
/// one file must not abort git's whole diff, so there a panic or an error is reported - one
/// `omnidiff: could not diff PATH` line in the output, the reason once on stderr - and the run
/// exits 0, as a binary file's notice would. Invoked directly, it exits 2 with the reason.
///
/// When the environment negotiates OSC 1717 (see `tui::osc1717`) and the output is text, its
/// handshake goes first, before anything can fail, so that every run that prints - and even one
/// that only errors - announces it.
fn run_contained(args: &Args, before: &Path, after: &Path) -> Result<i32> {
    tui::panic_guard::install_quiet_hook();
    let negotiated = !should_run_json(args) && osc1717::negotiated();
    if negotiated {
        tui::headless::write_stdout(osc1717::HANDSHAKE)?;
    }
    let record_file = record_path(&args.paths)
        .and_then(osc1717::file_field)
        .filter(|_| negotiated);
    let record_file = record_file.as_deref();
    contained(args, before, after, record_file, || {
        run_non_interactive(args, before, after, record_file)
    })
}

/// [`run_contained`]'s reporting around any `work`, for the tests.
fn contained(
    args: &Args,
    before: &Path,
    after: &Path,
    record_file: Option<&str>,
    work: impl FnOnce() -> Result<i32>,
) -> Result<i32> {
    let outcome = tui::panic_guard::contain(work);
    let git_form = invoked_as_git_external_diff(&args.paths);
    let name = if git_form {
        printable_path(&args.paths[0])
    } else {
        format!("{} and {}", printable_path(before), printable_path(after))
    };
    let reason = match outcome {
        Ok(Ok(code)) => return Ok(code),
        Ok(Err(error)) if is_broken_pipe(&error) || !git_form => return Err(error),
        Ok(Err(error)) => format!("{error:#}"),
        Err(panic) => format!("internal error: {panic}"),
    };
    eprintln!("omnidiff: could not diff {name}: {}", printable(&reason));
    if !git_form {
        return Ok(EXIT_FAILURE);
    }
    if !should_run_json(args) {
        let record = record_file.map_or(String::new(), osc1717::file_record);
        tui::headless::write_stdout(&format!("{record}omnidiff: could not diff {name}\n"))?;
    }
    Ok(0)
}

/// Writes a shell completion script or a man page to stdout, generated from the clap definition
/// so neither can drift from the real flags. The `packaging/` recipes redirect it.
fn run_util(action: &UtilAction) -> Result<()> {
    use clap::CommandFactory;

    let mut command = Args::command();
    match action {
        UtilAction::Completions { shell } => {
            // Explicit bin name: clap would otherwise use the crate name, which need not match.
            clap_complete::generate(*shell, &mut command, "omnidiff", &mut std::io::stdout());
        }
        UtilAction::Man => {
            clap_mangen::Man::new(command).render(&mut std::io::stdout())?;
        }
        UtilAction::Graphics => {
            // The terminal answers on stdin: raw, so the answer is neither echoed nor held for a
            // newline.
            crossterm::terminal::enable_raw_mode()?;
            let (picker, why) = omnidiff::tui::components::picture_viewer::detect_graphics();
            crossterm::terminal::disable_raw_mode()?;
            let protocol = picker.map_or("halfblocks".to_string(), |picker| {
                format!("{:?}", picker.protocol_type()).to_lowercase()
            });
            println!("{protocol}\t{why}");
        }
    }
    Ok(())
}

/// Whether `error` is the reader having closed stdout (`omnidiff a b | head`, a pager quit early).
/// That is the ordinary end of a run, not a failure to report.
fn is_broken_pipe(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::BrokenPipe)
    })
}

/// Every failure exits [`EXIT_FAILURE`] with an `omnidiff: ...` line, whichever path raised it;
/// clap's own usage errors already exit 2 on their own. The one exception is a broken pipe, which
/// exits 0 without a word.
///
/// A plain `fn main`: only the TUI awaits anything, so only it builds a tokio runtime
/// ([`run_tui`]). git runs one process per changed file under `GIT_EXTERNAL_DIFF`, and a
/// multi-threaded runtime's start-up and shutdown would be paid for each of them.
fn main() {
    let args = parse_args(std::env::args_os().collect(), under_git_external_diff());
    let code = match run(&args) {
        Ok(code) => code,
        Err(error) if is_broken_pipe(&error) => 0,
        Err(error) => {
            // An error's message often quotes a path, which may be a hostile commit's.
            eprintln!("omnidiff: {}", printable(&format!("{error:#}")));
            EXIT_FAILURE
        }
    };
    std::process::exit(code);
}

/// Whether git is running this process as an external diff. git sets `GIT_DIFF_PATH_COUNTER` and
/// `GIT_DIFF_PATH_TOTAL` for every `GIT_EXTERNAL_DIFF`/`diff.external` command (git(1)).
fn under_git_external_diff() -> bool {
    std::env::var_os("GIT_DIFF_PATH_TOTAL").is_some()
}

/// `Args` from `argv`. Under `GIT_EXTERNAL_DIFF` git appends its 7 or 9 arguments after the
/// configured command, with no `--` before them, so a file named `-x.rs` or `--help` at the root
/// of a hostile commit would be parsed as a flag: clap exits 2, and git abandons the entire diff.
/// So when git is the caller and the arguments end in that shape, a `--` goes in front of them
/// and they stay paths; the configured command's own flags before them are parsed as usual.
fn parse_args(argv: Vec<std::ffi::OsString>, under_git: bool) -> Args {
    Args::parse_from(with_git_arguments_positional(argv, under_git))
}

/// [`parse_args`]' `--` insertion, on its own for the tests.
fn with_git_arguments_positional(
    mut argv: Vec<std::ffi::OsString>,
    under_git: bool,
) -> Vec<std::ffi::OsString> {
    if !under_git {
        return argv;
    }
    for count in [9, 7] {
        // `argv[0]` is the program.
        let Some(start) = argv.len().checked_sub(count).filter(|&start| start >= 1) else {
            continue;
        };
        if is_git_external_diff_tail(&argv[start..]) {
            if !argv[1..start].iter().any(|arg| arg == "--") {
                argv.insert(start, "--".into());
            }
            break;
        }
    }
    argv
}

/// Whether `tail` (7 or 9 arguments) is git's `path old-file old-hex old-mode new-file new-hex
/// new-mode [new-path header]`: the hashes hex object names and the modes six octal digits, each
/// `.` for a side git has no version of. A file name could pose as one, never as all four.
///
/// A hash is 7 to 64 hex digits: git itself passes full names (40 for SHA-1, 64 for SHA-256), but
/// callers that imitate it abbreviate - lazygit probes a diff renderer with `0000000`.
fn is_git_external_diff_tail(tail: &[std::ffi::OsString]) -> bool {
    let text = |index: usize| tail[index].to_str().unwrap_or("");
    let hash = |index: usize| {
        let hash = text(index);
        hash == "."
            || ((7..=64).contains(&hash.len()) && hash.bytes().all(|b| b.is_ascii_hexdigit()))
    };
    let mode = |index: usize| {
        let mode = text(index);
        mode == "." || (mode.len() == 6 && mode.bytes().all(|b| (b'0'..=b'7').contains(&b)))
    };
    hash(2) && mode(3) && hash(5) && mode(6)
}

/// The whole program; the `Ok` value is the exit status of a completed run.
fn run(args: &Args) -> Result<i32> {
    if let Some(Command::Git {
        action: GitAction::Configure,
    }) = &args.command
    {
        return git_configure::run().map(|()| 0);
    }

    if let Some(Command::Jj {
        action: JjAction::Configure,
    }) = &args.command
    {
        return jj_configure::run().map(|()| 0);
    }

    if let Some(Command::Util { action }) = &args.command {
        return run_util(action).map(|()| 0);
    }

    let before_after = resolve_before_after(&args.paths)?;
    let stdout_is_terminal = std::io::stdout().is_terminal();
    let non_interactive = should_run_json(args) || should_run_headless(args, stdout_is_terminal);

    if let Some((before, after)) = before_after.as_ref() {
        if non_interactive {
            return run_contained(args, before, after);
        }
        // A pair diffed by content goes on to the TUI's content view; every other binary pair is
        // answered here.
        let pair = Pair::read(before, after)?;
        if let PairContents::Binary {
            before: before_bytes,
            after: after_bytes,
        } = &pair.contents
            && omnidiff::diff::content::pair_kind(before_bytes, after_bytes).is_none()
        {
            return run_binary(args, before, after, (before_bytes, after_bytes), None);
        }
    }

    if should_run_json(args) {
        before_after
            .as_ref()
            .context("`--mode json` needs BEFORE and AFTER - pass two files to diff")?;
    }
    if should_run_headless(args, stdout_is_terminal) {
        before_after
            .as_ref()
            .context(headless_needs_files_message(args))?;
    }

    run_tui(args, before_after)?;
    Ok(0)
}

/// The TUI, on the tokio runtime it needs: multi-threaded, as `#[tokio::main]` builds it.
fn run_tui(args: &Args, before_after: Option<(PathBuf, PathBuf)>) -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("cannot start the TUI's runtime")?
        .block_on(tui_main(args, before_after))
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnidiff::tui::positional::picture_notice;

    fn os(args: &[&str]) -> Vec<std::ffi::OsString> {
        args.iter().map(Into::into).collect()
    }

    const HEX_A: &str = "1111111111111111111111111111111111111111";
    const HEX_B: &str = "2222222222222222222222222222222222222222";

    /// git passes its arguments without `--`, so a root file named like a flag would be parsed
    /// as one and the whole `git diff` aborted.
    #[test]
    fn gits_arguments_stay_paths_whatever_the_file_is_called() {
        for name in ["-x.rs", "--help", "--mode=json", "git", "util"] {
            let argv = os(&[
                "omnidiff",
                "--headless",
                name,
                "/tmp/a",
                HEX_A,
                "100644",
                "/tmp/b",
                HEX_B,
                "100644",
            ]);
            let fixed = with_git_arguments_positional(argv.clone(), true);
            assert_eq!(fixed[2], "--", "{name}");
            let args = Args::try_parse_from(fixed).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(args.headless && args.command.is_none(), "{name}");
            assert_eq!(args.paths[0], PathBuf::from(name));
            assert_eq!(args.paths.len(), 7);
            assert_eq!(
                with_git_arguments_positional(argv.clone(), false),
                argv,
                "untouched when git is not the caller"
            );
        }
    }

    #[test]
    fn a_rename_and_an_added_file_are_recognised_as_gits_too() {
        let rename = os(&[
            "omnidiff",
            "-x.rs",
            "/tmp/a",
            HEX_A,
            "100644",
            "/tmp/b",
            HEX_B,
            "100755",
            "-y.rs",
            "similarity index 90%\n",
        ]);
        let fixed = with_git_arguments_positional(rename, true);
        assert_eq!(fixed[1], "--");
        assert_eq!(Args::try_parse_from(fixed).unwrap().paths.len(), 9);
        let added = os(&[
            "omnidiff",
            "-new.rs",
            "/dev/null",
            ".",
            ".",
            "/tmp/b",
            HEX_B,
            "100644",
        ]);
        assert_eq!(with_git_arguments_positional(added, true)[1], "--");
    }

    /// `git difftool` runs its helper as the external diff, so `GIT_DIFF_PATH_TOTAL` reaches a
    /// difftool command too: two paths, which are not git's shape and are left alone - as is a
    /// command that already ends options with `--`.
    #[test]
    fn anything_else_under_git_is_parsed_as_given() {
        let difftool = os(&["omnidiff", "--", "/tmp/a.rs", "-b.rs"]);
        assert_eq!(
            with_git_arguments_positional(difftool.clone(), true),
            difftool
        );
        let already = os(&[
            "omnidiff", "--", "-x.rs", "/tmp/a", HEX_A, "100644", "/tmp/b", HEX_B, "100644",
        ]);
        assert_eq!(
            with_git_arguments_positional(already.clone(), true),
            already
        );
        let not_hex = os(&["omnidiff", "a", "b", "c", "d", "e", "f", "g"]);
        assert_eq!(
            with_git_arguments_positional(not_hex.clone(), true),
            not_hex
        );
    }

    /// lazygit probes a diff renderer as git would run it, but with abbreviated hashes; 7 to 64
    /// hex digits are git's shape, fewer or more are not.
    #[test]
    fn abbreviated_hashes_are_gits_shape_too() {
        let tail = |hash: &str| os(&["probe", "/tmp/a", hash, "100644", "/tmp/b", hash, "100644"]);
        for hash in ["0000000", "abcdef0", HEX_A, &"f".repeat(64)] {
            assert!(is_git_external_diff_tail(&tail(hash)), "{hash}");
        }
        for hash in ["000000", &"f".repeat(65), "000000g"] {
            assert!(!is_git_external_diff_tail(&tail(hash)), "{hash}");
        }
        let mut probe = os(&["omnidiff", "--context", "3"]);
        probe.extend(tail("0000000"));
        assert_eq!(with_git_arguments_positional(probe, true)[3], "--");
    }

    /// git's argument form never opens the viewer, even on a terminal: git (`git --no-pager
    /// diff`, lazygit's pseudo-terminal) waits for each file's text. Only `--mode tui` asks for it.
    #[test]
    fn gits_argument_form_runs_headless_on_a_terminal_unless_the_tui_is_asked_for() {
        let git_form = [
            "f.rs", "/tmp/a", "0000000", "100644", "/tmp/b", "0000000", "100644",
        ];
        let mut argv = vec!["omnidiff"];
        argv.extend(git_form);
        let args = Args::try_parse_from(os(&argv)).unwrap();
        assert!(should_run_headless(&args, true));

        let mut argv = vec!["omnidiff", "--mode", "tui"];
        argv.extend(git_form);
        let args = Args::try_parse_from(os(&argv)).unwrap();
        assert!(!should_run_headless(&args, true));
        assert!(
            should_run_headless(&args, false),
            "a pipe still gets text whatever the mode"
        );

        let difftool = Args::try_parse_from(os(&["omnidiff", "a.rs", "b.rs"])).unwrap();
        assert!(
            !should_run_headless(&difftool, true),
            "git difftool keeps the viewer"
        );
    }

    /// A panic in the decoders or the diff must not abort git's whole diff: exit 0 there, and 2
    /// when invoked directly.
    #[test]
    fn a_panic_while_diffing_exits_0_under_git_and_2_directly() {
        let git = Args::try_parse_from(os(&[
            "omnidiff", "--mode", "json", "f.rs", "/tmp/a", HEX_A, "100644", "/tmp/b", HEX_B,
            "100644",
        ]))
        .unwrap();
        let (before, after) = (Path::new("/tmp/a"), Path::new("/tmp/b"));
        let code = contained(&git, before, after, None, || panic!("decoder bug"));
        assert_eq!(code.unwrap(), 0);
        let code = contained(&git, before, after, None, || anyhow::bail!("no AST"));
        assert_eq!(
            code.unwrap(),
            0,
            "an error is no more fatal there than a panic"
        );

        let direct = Args::try_parse_from(os(&["omnidiff", "--headless", "a", "b"])).unwrap();
        let code = contained(&direct, before, after, None, || panic!("decoder bug"));
        assert_eq!(code.unwrap(), EXIT_FAILURE);
        assert!(contained(&direct, before, after, None, || anyhow::bail!("no AST")).is_err());
        assert_eq!(
            contained(&direct, before, after, None, || Ok(1)).unwrap(),
            1
        );
    }

    #[test]
    fn resolve_before_after_with_no_args_starts_an_empty_viewer() {
        assert_eq!(resolve_before_after(&[]).unwrap(), None);
    }

    #[test]
    fn resolve_before_after_with_two_args_is_before_after_directly() {
        let paths = vec![PathBuf::from("before.rs"), PathBuf::from("after.rs")];
        assert_eq!(
            resolve_before_after(&paths).unwrap(),
            Some((PathBuf::from("before.rs"), PathBuf::from("after.rs")))
        );
    }

    #[test]
    fn resolve_before_after_with_seven_args_picks_out_old_file_and_new_file() {
        let paths = vec![
            PathBuf::from("src/foo.rs"),
            PathBuf::from("/tmp/git-blob-AAAA/foo.rs"),
            PathBuf::from("abc123"),
            PathBuf::from("100644"),
            PathBuf::from("/tmp/git-blob-BBBB/foo.rs"),
            PathBuf::from("def456"),
            PathBuf::from("100644"),
        ];
        assert_eq!(
            resolve_before_after(&paths).unwrap(),
            Some((
                PathBuf::from("/tmp/git-blob-AAAA/foo.rs"),
                PathBuf::from("/tmp/git-blob-BBBB/foo.rs")
            ))
        );
    }

    /// git appends the new path and a similarity score for a rename or copy, and `diff.renames`
    /// is on by default, so rejecting this form would abort the whole `git diff`.
    #[test]
    fn resolve_before_after_accepts_gits_nine_argument_rename_form() {
        let paths = vec![
            PathBuf::from("README.md"),
            PathBuf::from("/tmp/git-blob-AAAA/README.md"),
            PathBuf::from("abc123"),
            PathBuf::from("100644"),
            PathBuf::from("/tmp/git-blob-BBBB/README_tmp.md"),
            PathBuf::from("abc123"),
            PathBuf::from("100644"),
            PathBuf::from("README_tmp.md"),
            PathBuf::from("similarity index 100%"),
        ];
        assert_eq!(
            resolve_before_after(&paths).unwrap(),
            Some((
                PathBuf::from("/tmp/git-blob-AAAA/README.md"),
                PathBuf::from("/tmp/git-blob-BBBB/README_tmp.md")
            ))
        );
        assert!(invoked_as_git_external_diff(&paths));
    }

    /// git's temp files go in a new random directory each run (issue #59), so the titles name
    /// the path being diffed instead: argument 1 for both sides.
    #[test]
    fn display_names_under_git_external_diff_are_the_path_being_diffed() {
        let paths = vec![
            PathBuf::from("src/foo.rs"),
            PathBuf::from("/tmp/git-blob-AAAA/foo.rs"),
            PathBuf::from("abc123"),
            PathBuf::from("100644"),
            PathBuf::from("/tmp/git-blob-BBBB/foo.rs"),
            PathBuf::from("def456"),
            PathBuf::from("100644"),
        ];
        assert_eq!(
            display_names(&paths),
            Some((PathBuf::from("src/foo.rs"), PathBuf::from("src/foo.rs")))
        );
    }

    /// A rename or copy names the old path before and argument 8, the new path, after.
    #[test]
    fn display_names_of_a_rename_are_the_old_and_the_new_path() {
        let paths = vec![
            PathBuf::from("README.md"),
            PathBuf::from("/tmp/git-blob-AAAA/README.md"),
            PathBuf::from("abc123"),
            PathBuf::from("100644"),
            PathBuf::from("/tmp/git-blob-BBBB/README_tmp.md"),
            PathBuf::from("abc123"),
            PathBuf::from("100644"),
            PathBuf::from("README_tmp.md"),
            PathBuf::from("similarity index 100%"),
        ];
        assert_eq!(
            display_names(&paths),
            Some((PathBuf::from("README.md"), PathBuf::from("README_tmp.md")))
        );
    }

    /// An added file's missing side is named `/dev/null`, as `git diff` labels it.
    #[test]
    fn display_names_keep_dev_null_for_an_added_file() {
        let paths = vec![
            PathBuf::from("src/foo.rs"),
            PathBuf::from("/dev/null"),
            PathBuf::from("."),
            PathBuf::from("."),
            PathBuf::from("src/foo.rs"),
            PathBuf::from("def456"),
            PathBuf::from("100644"),
        ];
        assert_eq!(
            display_names(&paths),
            Some((PathBuf::from("/dev/null"), PathBuf::from("src/foo.rs")))
        );
    }

    /// Git for Windows hands a missing side over as `nul`, still marked by its `.` hash and
    /// mode; the title is `/dev/null` all the same.
    #[test]
    fn display_names_name_a_deleted_files_nul_side_dev_null() {
        let paths = vec![
            PathBuf::from("src/foo.rs"),
            PathBuf::from("C:/Temp/git-blob-AAAA/foo.rs"),
            PathBuf::from("abc123"),
            PathBuf::from("100644"),
            PathBuf::from("nul"),
            PathBuf::from("."),
            PathBuf::from("."),
        ];
        assert_eq!(
            display_names(&paths),
            Some((PathBuf::from("src/foo.rs"), PathBuf::from("/dev/null")))
        );
    }

    /// `BEFORE AFTER` already names the files the user gave.
    #[test]
    fn display_names_of_two_paths_are_the_paths_themselves() {
        let paths = vec![PathBuf::from("before.rs"), PathBuf::from("after.rs")];
        assert_eq!(display_names(&paths), None);
    }

    #[test]
    fn resolve_before_after_still_rejects_an_unrecognized_argument_count() {
        let paths: Vec<PathBuf> = (0..8).map(|n| PathBuf::from(n.to_string())).collect();
        assert!(resolve_before_after(&paths).is_err());
        assert!(!invoked_as_git_external_diff(&paths));
    }

    /// An added or deleted file arrives as `/dev/null`; it is passed through, not special-cased.
    #[test]
    fn resolve_before_after_with_seven_args_passes_dev_null_through_for_add_delete() {
        let paths = vec![
            PathBuf::from("src/foo.rs"),
            PathBuf::from("/dev/null"),
            PathBuf::from("."),
            PathBuf::from("."),
            PathBuf::from("/tmp/git-blob-BBBB/foo.rs"),
            PathBuf::from("def456"),
            PathBuf::from("100644"),
        ];
        assert_eq!(
            resolve_before_after(&paths).unwrap(),
            Some((
                PathBuf::from("/dev/null"),
                PathBuf::from("/tmp/git-blob-BBBB/foo.rs")
            ))
        );
    }

    fn args_with(mode: Mode, headless: bool) -> Args {
        Args {
            command: None,
            paths: Vec::new(),
            // As clap leaves it when `--mode` is not given.
            mode: (mode != Mode::Tui).then_some(mode),
            headless,
            minimal: false,
            full: false,
            whole_updates: false,
            paint_reindent_moves: false,
            color: ColorChoice::Auto,
            exit_code: false,
            context: tui::headless::CONTEXT_LINES,
            review: false,
            tui_tick_rate: 4.0,
            tui_frame_rate: 60.0,
        }
    }

    /// Without any render flag the saved `M` panel setting applies untouched, "whole updates"
    /// included.
    #[test]
    fn no_render_flag_leaves_the_saved_setting_alone() {
        assert_eq!(render_option_flags(&args_with(Mode::Tui, false)), None);

        let config = tempfile::NamedTempFile::new().expect("temp config");
        unsafe { std::env::set_var(omnidiff::tui::theme::CONFIG_ENV, config.path()) };
        omnidiff::tui::theme::save_render_options(omnidiff::diff::text::RenderOptions {
            whole_pair_updates: true,
            ..omnidiff::diff::text::RenderOptions::FULL
        });
        let options = render_options(&args_with(Mode::Headless, false));
        unsafe { std::env::remove_var(omnidiff::tui::theme::CONFIG_ENV) };

        assert!(options.whole_pair_updates);
    }

    /// `--review` with a pair, and `--headless` with `--mode json`, are errors rather than one flag
    /// silently winning.
    #[test]
    fn contradictory_mode_flags_are_rejected() {
        for argv in [
            &["omnidiff", "--review", "a.rs", "b.rs"][..],
            &["omnidiff", "--review", "--mode", "json"],
            &["omnidiff", "--headless", "--mode", "json"],
        ] {
            assert!(Args::try_parse_from(argv).is_err(), "{argv:?} was accepted");
        }
        assert!(Args::try_parse_from(["omnidiff", "--headless", "a.rs", "b.rs"]).is_ok());
        assert!(Args::try_parse_from(["omnidiff", "--review"]).is_ok());
    }

    #[test]
    fn whole_updates_flag_layers_onto_minimal_and_full_alike() {
        let mut minimal = args_with(Mode::Tui, false);
        minimal.minimal = true;
        minimal.whole_updates = true;
        assert!(render_options(&minimal).whole_pair_updates);
        assert!(!render_options(&minimal).leading_whitespace);

        let mut full = args_with(Mode::Tui, false);
        full.full = true;
        full.whole_updates = true;
        assert!(render_options(&full).whole_pair_updates);
        assert!(render_options(&full).leading_whitespace);
    }

    #[test]
    fn paint_reindent_moves_flag_layers_onto_minimal_and_full_alike() {
        let mut minimal = args_with(Mode::Tui, false);
        minimal.minimal = true;
        minimal.paint_reindent_moves = true;
        assert!(render_options(&minimal).paint_reindent_only_moves);
        assert!(!render_options(&minimal).leading_whitespace);

        let mut full = args_with(Mode::Tui, false);
        full.full = true;
        full.paint_reindent_moves = true;
        assert!(render_options(&full).paint_reindent_only_moves);
        assert!(render_options(&full).leading_whitespace);

        let mut without_the_flag = args_with(Mode::Tui, false);
        without_the_flag.minimal = true;
        assert!(
            !render_options(&without_the_flag).paint_reindent_only_moves,
            "omitting the flag must not touch this field"
        );
    }

    #[test]
    fn color_flag_parses_all_three_choices_and_defaults_to_auto() {
        for (argv, expected) in [
            (vec!["omnidiff"], ColorChoice::Auto),
            (vec!["omnidiff", "--color", "always"], ColorChoice::Always),
            (vec!["omnidiff", "--color", "never"], ColorChoice::Never),
            (vec!["omnidiff", "--color", "auto"], ColorChoice::Auto),
        ] {
            let args = Args::try_parse_from(argv.clone())
                .unwrap_or_else(|e| panic!("{argv:?} should parse: {e}"));
            assert_eq!(args.color, expected, "for {argv:?}");
        }
    }

    #[test]
    fn context_flag_defaults_to_the_headless_default_and_accepts_overrides() {
        assert_eq!(
            Args::try_parse_from(["omnidiff"]).unwrap().context,
            tui::headless::CONTEXT_LINES
        );
        assert_eq!(
            Args::try_parse_from(["omnidiff", "--context", "0"])
                .unwrap()
                .context,
            0
        );
    }

    /// A `--mode` typo is an error, not a silent fallback to the TUI; the values are also what
    /// shell completion offers.
    #[test]
    fn mode_accepts_its_three_values_case_insensitively_and_nothing_else() {
        for (value, expected) in [
            ("tui", Mode::Tui),
            ("TUI", Mode::Tui),
            ("headless", Mode::Headless),
            ("Headless", Mode::Headless),
            ("json", Mode::Json),
            ("JSON", Mode::Json),
        ] {
            let args = Args::try_parse_from(["omnidiff", "--mode", value]).unwrap();
            assert_eq!(args.mode, Some(expected), "--mode {value}");
        }
        assert!(Args::try_parse_from(["omnidiff", "--mode", "bogus"]).is_err());
        assert!(Args::try_parse_from(["omnidiff", "--exact"]).is_err());
    }

    /// `Duration::from_secs_f64(1.0 / rate)` panics on these; they are argument errors instead.
    #[test]
    fn tui_rates_reject_zero_negative_and_non_numbers() {
        for value in ["0", "-1", "nan", "inf", "fast"] {
            assert!(
                Args::try_parse_from(["omnidiff", "--tui-tick-rate", value]).is_err(),
                "--tui-tick-rate {value}"
            );
            assert!(
                Args::try_parse_from(["omnidiff", "--tui-frame-rate", value]).is_err(),
                "--tui-frame-rate {value}"
            );
        }
        let args = Args::try_parse_from(["omnidiff", "--tui-tick-rate", "2.5"]).unwrap();
        assert_eq!(args.tui_tick_rate, 2.5);
    }

    /// The rate flags are development knobs, hidden from `--help` and the man page.
    #[test]
    fn tui_rates_are_hidden_from_help() {
        use clap::CommandFactory;

        let help = Args::command().render_long_help().to_string();
        assert!(!help.contains("--tui-tick-rate"), "{help}");
        assert!(!help.contains("--tui-frame-rate"), "{help}");
        assert!(help.contains("--mode"), "{help}");
    }

    /// no-color.org: only a non-empty value opts out.
    #[test]
    fn no_color_opts_out_only_when_set_and_non_empty() {
        assert!(!no_color_is_set(None));
        assert!(!no_color_is_set(Some("".into())));
        assert!(no_color_is_set(Some("1".into())));
    }

    #[test]
    fn the_no_files_message_names_the_missing_files_when_text_mode_was_asked_for() {
        assert!(headless_needs_files_message(&args_with(Mode::Tui, true)).starts_with("text mode"));
        assert!(
            headless_needs_files_message(&args_with(Mode::Headless, false))
                .starts_with("text mode")
        );
        assert!(
            headless_needs_files_message(&args_with(Mode::Tui, false))
                .starts_with("stdout is not a terminal")
        );
    }

    #[test]
    fn a_broken_pipe_anywhere_in_the_chain_is_recognized() {
        let io = std::io::Error::from(std::io::ErrorKind::BrokenPipe);
        assert!(is_broken_pipe(&anyhow::Error::from(io).context("writing")));
        let other = std::io::Error::from(std::io::ErrorKind::NotFound);
        assert!(!is_broken_pipe(&anyhow::Error::from(other)));
    }

    #[test]
    fn always_and_never_beat_the_no_color_environment_variable() {
        // Always/Never never read the environment, so no env mutation (which races) is needed.
        assert!(use_color(ColorChoice::Always));
        assert!(!use_color(ColorChoice::Never));
    }

    #[test]
    fn without_the_flag_a_differing_pair_still_exits_zero() {
        assert_eq!(exit_code_for(true, false, false), 0);
        assert_eq!(exit_code_for(false, false, false), 0);
    }

    #[test]
    fn with_the_flag_exit_codes_follow_the_diff_convention() {
        assert_eq!(exit_code_for(false, true, false), 0);
        assert_eq!(exit_code_for(true, true, false), 1);
    }

    #[test]
    fn the_git_external_diff_form_never_returns_one_even_with_the_flag() {
        assert_eq!(exit_code_for(true, true, true), 0);
        assert_eq!(exit_code_for(false, true, true), 0);
    }

    #[test]
    fn a_binary_pair_under_the_git_external_diff_form_still_exits_zero() {
        assert_eq!(exit_code_for(true, false, true), 0);
        assert_eq!(exit_code_for(true, true, true), 0);
    }

    #[test]
    fn the_binary_notice_names_gits_logical_path_not_its_temp_blobs() {
        let paths = vec![
            PathBuf::from("doc/paper.pdf"),
            PathBuf::from("/tmp/git-blob-AAAA/paper.pdf"),
            PathBuf::from("abc123"),
            PathBuf::from("100644"),
            PathBuf::from("/tmp/git-blob-BBBB/paper.pdf"),
            PathBuf::from("def456"),
            PathBuf::from("100644"),
        ];
        let (before, after) = resolve_before_after(&paths).unwrap().unwrap();
        assert_eq!(
            binary_notice(&paths, &before, &after, true),
            "Binary file doc/paper.pdf differs\n"
        );
    }

    #[test]
    fn the_binary_notice_names_both_sides_for_the_two_argument_form() {
        let paths = vec![PathBuf::from("old.pdf"), PathBuf::from("new.pdf")];
        let (before, after) = resolve_before_after(&paths).unwrap().unwrap();
        assert_eq!(
            binary_notice(&paths, &before, &after, true),
            "Binary files old.pdf and new.pdf differ\n"
        );
    }

    #[test]
    fn the_binary_notice_does_not_claim_identical_files_differ() {
        let paths = vec![PathBuf::from("a.pdf"), PathBuf::from("b.pdf")];
        let (before, after) = resolve_before_after(&paths).unwrap().unwrap();
        assert_eq!(
            binary_notice(&paths, &before, &after, false),
            "Binary files a.pdf and b.pdf are identical\n"
        );
    }

    fn picture_diff(
        comparison: omnidiff::diff::picture::Comparison,
    ) -> omnidiff::diff::picture::PictureDiff {
        let side = |width| omnidiff::diff::picture::PictureInfo {
            format: "PNG".to_string(),
            width,
            height: 10,
            color: "RGBA8".to_string(),
            bytes: 100,
            frames: 1,
            duration_ms: 0,
        };
        omnidiff::diff::picture::PictureDiff {
            before: Some(side(20)),
            after: Some(side(20)),
            comparison,
        }
    }

    #[test]
    fn the_picture_notice_says_how_much_changed_and_where_under_gits_name() {
        use omnidiff::diff::picture::{Comparison, Region};
        let paths = vec![
            PathBuf::from("assets/logo.png"),
            PathBuf::from("/tmp/git-blob-AAAA/logo.png"),
            PathBuf::from("abc123"),
            PathBuf::from("100644"),
            PathBuf::from("/tmp/git-blob-BBBB/logo.png"),
            PathBuf::from("def456"),
            PathBuf::from("100644"),
        ];
        let (before, after) = resolve_before_after(&paths).unwrap().unwrap();
        let region = Region {
            x: 6,
            y: 2,
            width: 4,
            height: 3,
            changed_pixels: 12,
        };
        let diff = picture_diff(Comparison::Pixels {
            changed_pixels: 12,
            total_pixels: 200,
            regions: vec![region],
            differing_pixels: 12,
            strong_pixels: 12,
            layout_kept: true,
        });
        assert_eq!(
            picture_notice(&paths, &before, &after, &diff),
            "Picture assets/logo.png: PNG 20x10 RGBA8, 100 bytes -> PNG 20x10 RGBA8, 100 bytes\n  \
             6.00% of pixels changed (12 of 200), in 1 region:\n    4x3 at (6, 2)\n"
        );
    }

    #[test]
    fn the_picture_notice_lists_an_animations_changed_added_and_removed_frames() {
        use omnidiff::diff::picture::{Comparison, FrameStep, Region};
        let paths = vec![PathBuf::from("a.gif"), PathBuf::from("b.gif")];
        let (before, after) = resolve_before_after(&paths).unwrap().unwrap();
        let mut diff = picture_diff(Comparison::Frames {
            steps: vec![
                FrameStep::Same {
                    before: 0,
                    after: 0,
                    frames: 4,
                },
                FrameStep::Changed {
                    before: 4,
                    after: 4,
                    changed_pixels: 3,
                    regions: vec![Region {
                        x: 1,
                        y: 1,
                        width: 3,
                        height: 1,
                        changed_pixels: 3,
                    }],
                    strong_pixels: 3,
                },
                FrameStep::Inserted {
                    after: 5,
                    frames: 2,
                },
                FrameStep::Deleted {
                    before: 5,
                    frames: 1,
                },
            ],
            total_pixels: 200,
            retimed: true,
            faint: false,
        });
        for side in [&mut diff.before, &mut diff.after] {
            let info = side.as_mut().unwrap();
            (info.format, info.frames, info.duration_ms) = ("GIF".to_string(), 6, 1500);
        }
        assert_eq!(
            picture_notice(&paths, &before, &after, &diff),
            "Pictures a.gif and b.gif: GIF 20x10 RGBA8, 6 frames, 1.5s, 100 bytes -> GIF 20x10 \
             RGBA8, 6 frames, 1.5s, 100 bytes\n  1 frame changed, 2 added, 1 removed; frames that \
             look the same show for different times\n    frame 4 -> 4: changed, in 1 region\n    \
             frames 5-6 added\n    frame 5 removed\n"
        );
    }

    #[test]
    fn the_archive_notice_lists_what_changed_in_which_member() {
        let zip = |files: &[(&str, &str)]| {
            let mut bytes = Vec::new();
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
            for (name, text) in files {
                writer
                    .start_file(*name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                std::io::Write::write_all(&mut writer, text.as_bytes()).unwrap();
            }
            writer.finish().unwrap();
            bytes
        };
        let before = zip(&[("a.txt", "1\n2\n"), ("gone.txt", "g\n"), ("same", "s\n")]);
        let after = zip(&[("a.txt", "1\nTWO\n"), ("new.txt", "n\n"), ("same", "s\n")]);
        let diff = omnidiff::diff::content::diff(&before, &after)
            .unwrap()
            .expect("an archive pair");
        let paths = [PathBuf::from("a.zip"), PathBuf::from("b.zip")];
        let notice = content_notice(&paths, &paths[0], &paths[1], &diff);
        assert!(
            notice.starts_with("Archives a.zip and b.zip: ZIP, 3 members, "),
            "{notice}"
        );
        assert!(
            notice.contains("  1 changed, 1 added, 1 removed, 1 unchanged\n"),
            "{notice}"
        );
        assert!(
            notice.contains("    changed  a.txt: 1 line removed, 1 added\n"),
            "{notice}"
        );
        assert!(notice.contains("    removed  gone.txt\n"), "{notice}");
        assert!(notice.contains("    added    new.txt\n"), "{notice}");
    }

    #[test]
    fn the_picture_notice_says_when_nothing_visible_changed_or_the_size_did() {
        use omnidiff::diff::picture::Comparison;
        let paths = vec![PathBuf::from("a.png"), PathBuf::from("b.png")];
        let (before, after) = resolve_before_after(&paths).unwrap().unwrap();
        let unchanged = picture_diff(Comparison::Pixels {
            changed_pixels: 0,
            total_pixels: 200,
            regions: Vec::new(),
            differing_pixels: 0,
            strong_pixels: 0,
            layout_kept: true,
        });
        assert!(
            picture_notice(&paths, &before, &after, &unchanged).ends_with("  no pixel changed\n")
        );
        let reshaped = Comparison::Resized {
            aspect_kept: false,
            scaled: None,
        };
        let resized = picture_notice(&paths, &before, &after, &picture_diff(reshaped));
        assert!(
            resized.starts_with("Pictures a.png and b.png: "),
            "{resized}"
        );
        assert!(
            resized.ends_with("  resized to another shape, so not compared pixel by pixel\n"),
            "{resized}"
        );
        let rescaled = Comparison::Resized {
            aspect_kept: true,
            scaled: Some(Box::new(Comparison::Pixels {
                changed_pixels: 3,
                total_pixels: 200,
                regions: Vec::new(),
                differing_pixels: 3,
                strong_pixels: 0,
                layout_kept: true,
            })),
        };
        let resized = picture_notice(&paths, &before, &after, &picture_diff(rescaled));
        assert!(
            resized.ends_with("  resized; scaled to one size, 1.50% of pixels changed\n"),
            "{resized}"
        );
    }

    #[test]
    fn exit_code_flag_defaults_to_off_and_parses() {
        assert!(!Args::try_parse_from(["omnidiff"]).unwrap().exit_code);
        assert!(
            Args::try_parse_from(["omnidiff", "--exit-code"])
                .unwrap()
                .exit_code
        );
    }

    #[test]
    fn git_configure_parses_as_a_subcommand() {
        let args = Args::try_parse_from(["omnidiff", "git", "configure"]).unwrap();
        assert!(matches!(
            args.command,
            Some(Command::Git {
                action: GitAction::Configure
            })
        ));
        assert!(args.paths.is_empty());
    }

    #[test]
    fn two_paths_still_parse_as_paths_not_a_subcommand() {
        let args = Args::try_parse_from(["omnidiff", "a.rs", "b.rs"]).unwrap();
        assert!(args.command.is_none());
        assert_eq!(
            args.paths,
            vec![PathBuf::from("a.rs"), PathBuf::from("b.rs")]
        );
    }

    /// The `packaging/` recipes call these, so a rename breaks them silently.
    #[test]
    fn util_generates_completions_and_a_man_page() {
        let args = Args::try_parse_from(["omnidiff", "util", "completions", "bash"]).unwrap();
        assert!(matches!(
            args.command,
            Some(Command::Util {
                action: UtilAction::Completions {
                    shell: clap_complete::Shell::Bash
                }
            })
        ));

        let args = Args::try_parse_from(["omnidiff", "util", "man"]).unwrap();
        assert!(matches!(
            args.command,
            Some(Command::Util {
                action: UtilAction::Man
            })
        ));
    }

    #[test]
    fn the_help_text_describes_omnidiff_rather_than_a_doc_comment() {
        use clap::CommandFactory;

        let about = Args::command().get_about().unwrap().to_string();
        assert!(
            about.starts_with("Fast, robust, accurate content-aware diffing"),
            "{about}"
        );
        assert_eq!(
            Args::command().get_version(),
            Some(env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn no_args_still_opens_empty_viewer() {
        let args = Args::try_parse_from(["omnidiff"]).unwrap();
        assert!(args.command.is_none());
        assert!(args.paths.is_empty());
    }

    #[test]
    fn should_run_headless_when_stdout_is_not_a_terminal_even_without_any_flag() {
        assert!(should_run_headless(&args_with(Mode::Tui, false), false));
    }

    #[test]
    fn should_run_headless_is_false_on_a_real_terminal_with_no_flags() {
        assert!(!should_run_headless(&args_with(Mode::Tui, false), true));
    }

    #[test]
    fn should_run_headless_honors_the_headless_flag_even_on_a_real_terminal() {
        assert!(should_run_headless(&args_with(Mode::Tui, true), true));
    }

    #[test]
    fn batch_flag_is_a_clap_alias_for_headless() {
        let args = Args::try_parse_from(["omnidiff", "--batch"]).expect("--batch should parse");
        assert!(
            args.headless,
            "--batch should set the same field as --headless"
        );
    }

    #[test]
    fn should_run_headless_honors_mode_headless_case_insensitively() {
        assert!(should_run_headless(&args_with(Mode::Headless, false), true));
    }

    #[test]
    fn should_run_json_honors_mode_json_case_insensitively() {
        assert!(should_run_json(&args_with(Mode::Json, false)));
        assert!(should_run_json(&args_with(Mode::Json, false)));
        assert!(!should_run_json(&args_with(Mode::Tui, false)));
        assert!(!should_run_json(&args_with(Mode::Headless, false)));
    }

    #[test]
    fn should_run_json_is_false_on_a_non_terminal_stdout_unless_explicitly_asked_for() {
        assert!(!should_run_json(&args_with(Mode::Tui, false)));
    }

    #[test]
    fn resolve_before_after_rejects_any_other_count() {
        for n in [1, 3, 4, 5, 6, 8] {
            let paths: Vec<PathBuf> = (0..n).map(|i| PathBuf::from(format!("p{i}"))).collect();
            assert!(
                resolve_before_after(&paths).is_err(),
                "{n} positional arguments should be rejected"
            );
        }
    }
}
