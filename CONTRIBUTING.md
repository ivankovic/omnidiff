# Contributing

**At this time, to keep the development speed high, contributions are not accepted.**

## Technology

The product is written in Rust. The showcase's browser viewer (`assets/viewer/`) is plain
JavaScript, and the research and analysis scripts are Python.

OmniDiff stores user configuration, for example the active theme, on disk with `confy`. The
dataset-analysis tools in `src/bin/` use a separate SQLite database to store the stats that they
collect.

The UI is a terminal UI, written with the Ratatui and Crossterm libraries.

### UI design patterns

The UI uses the [Component architecture](https://ratatui.rs/concepts/application-patterns/component-architecture/).

Each component encapsulates its own state, event handlers, and rendering logic.

## Code quality

Format all code with `cargo fmt`, the standard Rust formatter. CI enforces this on every push and
pull request.

No Rust check errors are allowed. Run `cargo clippy` frequently. CI also enforces `cargo clippy`,
across all three Cargo feature configs (see "CI" below).

The Python under `research/`, `scripts/` and `assets/` has the same two halves, as `ruff format`
and `ruff check` - run both with `make lint-python`. `ruff` is the one tool neither a bare checkout
nor the Rust toolchain brings: install it once for every repository under your user with
`uv tool install ruff@0.16.4`, matching the version `.github/workflows/ci.yml` pins, or work inside
`nix develop`, whose devShell already has it. The rule set is pinned in the root `ruff.toml` rather
than left on ruff's defaults, for the reason that file gives.

### Comments describe how the code *is*

A comment explains what the code does and why it is that way. It does not narrate what the code
used to be, what was tried and reverted, or when either happened. "An earlier version derived this
from the renderer" and "widening this gate regressed 9 fixtures on 2026-08-14" both read as live
facts to someone skimming, and neither can be checked against the code in front of them.

Where the *reason* for a choice is that the alternative is worse, say so in the present tense and
leave the measurement out: **"treating every such node as a candidate anchors unrelated nodes whose
operators happen to hash-match"**, not "an early version did that and cost 9 fixtures". The corpus
grows, so a number frozen in a comment goes stale silently; `benchmark_optimal_solutions` and
`research/data/quality/` are where the current ones live, and they are regenerated rather than
remembered.

Two things this does **not** mean:

* Runtime state is not history. "an earlier hop in this chain", "the previously submitted query",
  "the old partner" describe what the program is doing now.
* A test whose subject is a past bug keeps its subject - stated as the property it pins. "Esc
  closes the theme picker rather than quitting", not "Esc used to quit the whole app".

`git log` and `git blame` hold the history.

## Testing

[src/test/README.md](src/test/README.md) describes how OmniDiff is tested: the fixture corpus and
its ground truth, the speed, robustness and accuracy goals with their current figures, coverage,
and the rules for writing tests (no mocks; per-file unit tests under 1 second, fixture tests under
5). Run automated tests frequently during coding, and `make benchmark-quality` to see what a change
did to accuracy and speed.

## Code structure

Follow Rust's standard project structure.

```
<root of the repository>
    |- /src             <- The implementation
        |- main.rs      <- Entry point: parses CLI args and starts the TUI
        |- code.rs      <- The struct and methods related to reading and parsing one unit of code
        |- code/        <- Sensible implementation units related to code.rs
        |- diff.rs      <- Everything related to actually diffing two or more units of code
        |- diff/        <- Sensible implementation units related to diff.rs
        |- stats.rs     <- Tools used to process large datasets to guide the design
        |- stats/       <- Sensible implementation units related to stats.rs
        |- tui.rs       <- Declares the TUI's submodules and sets up logging
        |- tui/         <- The TUI itself: app.rs (controller), ui.rs (terminal rendering),
        |                  components/, widgets/
        |- showcase.rs  <- The GitHub Pages showcase's data: the diff and first-frame state the
        |- showcase/       browser viewer is fed (feature `test-fixtures`)
        |- test/        <- Shared test helpers, plus slower fixture-driven tests (see its README.md)
        |- bin/         <- Standalone developer tools: benchmarking, dataset sampling, and more
    |- /assets/viewer   <- The showcase's browser viewer: model.js is the TUI's viewer logic ported
    |                      to the browser, app.js the DOM wiring
    |- /docs            <- How to install and use OmniDiff
    |- /packaging       <- Package-manager recipes and the release checklist
    |- /research        <- Datasets and analysis scripts used to guide design decisions
    |- README.md        <- High-level project summary, linking to the other READMEs. Must be
    |                      readable to humans.
    |- CONTRIBUTING.md  <- This file
    |- AGENTS.md        <- AI-only instructions
    |- REVIEW.md        <- Open code-health items
```

`README.md` files can exist in any subdirectory: a high-level summary of it, readable by humans.
Why the code is the way it is lives next to the code, in module-level doc comments (`//!`), not in
separate design documents. Working notes (follow-ups, experiments, negative results) are kept out
of the repository; `AGENT_LOG.md` at the root is git-ignored for that. `REVIEW.md` stays root-only.

## Makefile targets

These are the repository-root Makefile's main targets - product concerns only: build, test,
install, the quality gate, release. The corpus, measurement, analysis and paper targets live in
`research/Makefile`, are run from that directory (`cd research && make <target>`), and are
documented there.

### Build, test, quality

* `test` - `cargo nextest run --cargo-profile ci --all-features`, plus `test-mapping-site-js`,
  `test-viewer-js` and `test-showcase-js` (plain-Node tests of the human-mapping site's and the
  showcase's vanilla JS, which cargo's suite cannot cover - see the root Makefile) and
  `test-python` (`pytest` over the research and script helpers, in `research/`'s uv environment).
  Requires `cargo-nextest` (`cargo install cargo-nextest`, one-time), Node.js, and
  [`uv`](https://docs.astral.sh/uv/). Unlike `cargo test`, nextest
  runs each test in its own process rather than as a thread inside one long-lived binary, so the
  `src/test/helper.rs` fixture caches (never-evicting, process-lifetime) get reclaimed by the OS
  after every test instead of accumulating for the whole suite. Measured on this repo's full suite,
  same machine, both `--release`: peak RSS 10.66GB under plain `cargo test --release` vs 5.37GB
  under `cargo nextest run --release` - about half, at comparable wall-clock time.
* `coverage` - line coverage of the suite over this repository's own code, via `cargo-llvm-cov`
  driving nextest (`cargo install cargo-llvm-cov`, plus `rustup component add llvm-tools-preview`).
  See [Coverage](src/test/README.md#coverage) for what it reports and why it is not a gate.
* `test-quick` - the edit loop's Rust suite, about 12 s warm against `test-rust`'s two minutes:
  every test outside the fixtures but the corpus sweeps, and the fixtures
  `src/test/data/quick_fixtures.txt` names, the cheapest set reaching every line the whole fixture
  corpus reaches beyond those tests, plus any fixture added since that list was measured. Not a
  gate; `test` and CI run everything.
* `update-quick-fixtures` - rewrites that list from per-test coverage, measured first with
  `make -C research measure-per-test-coverage` (about ten minutes). Re-run after engine changes.
* `lint-python` - `ruff check` then `ruff format --check` over `research`, `scripts` and `assets`,
  the same three directories CI's python job covers. Requires `ruff`
  (`uv tool install ruff@0.16.4`, one-time - see "Code quality" above).
* `build` - `cargo build --release --features stats` (the `stats` feature builds the
  dataset-analysis binaries in `src/bin/`). It does not run the tests.
* `install` - `cargo install --path . --force`, so `omnidiff` on `PATH` matches this checkout.
* `install-hooks` - one-time setup that points git at `.githooks/`. `pre-commit` formats the Rust
  and Python a commit stages (`cargo fmt`, `ruff format`) and regenerates `src/test/data/diffs.csv`
  when a commit touches the fixture corpus. `pre-push` runs the fast subset of what CI checks
  (`cargo fmt --check`, a per-feature-config `cargo clippy`, `ruff`, the site JS tests) before a
  `git push` leaves your machine - see each file's own comment for exactly what it does and does not
  cover. `git push --no-verify` skips it for one push.
* `ci` - the whole of CI, locally: every job in `.github/workflows/ci.yml`, in that file's own
  order. Unlike the pre-push hook above it includes the release build, the full test suite for all
  three feature configs, and the quality gate, so it takes minutes rather than seconds - run it
  when you mean to push, not on every push. It reads the commands out of `ci.yml` itself rather
  than keeping a copy, so it cannot drift from CI; `python3 scripts/ci_local.py --list` shows the
  job ids and `--job <id>` runs one of them. See that script's module docstring for what it can
  and cannot mirror.

Three verbs, and which file a target lives in follows from them:

* **`benchmark-`** measures **omnidiff**, and lives in the root Makefile. `benchmark-quality`
  answers both questions a change raises - is it right, and is it fast - in one run over the
  fixture corpus. Production QA, and it needs nothing a bare checkout lacks.
* **`check-`** gates. Runs in CI on every push and fails the build. `check-quality` gates on
  precisely what `benchmark-quality` measures - the pairing is the point.
* **`measure-`** measures anything that is not omnidiff alone: other people's tools, or the cloned
  upstream corpus at `REPOSITORIES_DIR`. Lives in `research/Makefile`, never the root one. A number
  that moves when someone else ships a GumTree release is a study of the field, not product QA.

* `benchmark-quality` - runs `benchmark_optimal_solutions`: mismatch count against the
  human-authored ground truth, per fixture, and the runtime (see
  [Quality](src/test/README.md#quality) and [Speed](src/test/README.md#speed)).
* `benchmark-ablation` - re-runs `benchmark-quality` with individual solver passes disabled, to see
  what each is worth. A one-off investigation rather than a routine measurement, which is why it is
  not folded into `benchmark-quality`.
* `check-quality` - the gate. What CI runs on every push and what `deploy` runs before it tags a
  release. Fails hard on an accuracy regression against the checked-in baseline; only warns on a
  runtime jump of more than 2x.
* `update-quality-baseline` - re-cuts the accuracy baseline and this machine's timing baseline
  after a reviewed change; `update-timing-baseline` re-cuts the timing alone. `deploy` never runs
  it automatically. It cannot lower the accuracy bar: a test pins `quality_baseline.csv`'s
  per-fixture accuracy columns to the `fixtures` stubs' limits, so raising a limit means editing
  that fixture's stub - the same file that holds the prose explaining why.

### Release

* `deploy` - publishes a release everywhere: `deploy-crates` then `deploy-github`, in that order
  (crates.io first, since a publish there can never be undone - only yanked - while a GitHub tag
  and Release are trivial to redo). Both refuse to run on a dirty working tree, on a `HEAD` that
  does not match `origin/main`, on a packaging recipe that names a version other than
  `Cargo.toml`'s (`check-versions`), or on a `check-quality` regression (`deploy-checks`, shared
  by both - a plain `make deploy` only pays for it once). The step-by-step list, including what
  can only be done after the tag exists, is the release checklist in `packaging/README.md`.
* `check-versions` - `scripts/check_version_sync.py`: the AUR, Gentoo and Nix recipes repeat
  `Cargo.toml`'s version by hand, and this fails if any of them disagree.
* `deploy-crates` - publishes the current `Cargo.toml` version to crates.io (`cargo publish
  --locked`). Requires `cargo login` to already be configured locally (or `CARGO_REGISTRY_TOKEN`
  set).
* `deploy-github` - tags the current commit `v<Cargo.toml version>` and pushes the tag. This push
  triggers `.github/workflows/release.yml`, which creates a draft GitHub Release whose notes are
  the version's section of `CHANGELOG.md` (it fails if that section is missing or still says
  `unreleased`), attaches the cross-platform `omnidiff` binaries, the `.deb`s, completions and
  checksums, and publishes the release once every asset is there.

## CI

Every push and pull request runs (see `.github/workflows/ci.yml`):

* `cargo fmt --check`
* `cargo clippy --tests -- -D warnings`, once each for the three Cargo feature configs (default,
  `test-fixtures`, `stats` - see Cargo.toml's `[features]`)
* `cargo build` + `cargo nextest run` under the `ci` profile, once each for the same three
  feature configs; the fixture-corpus tests, which no feature changes, run once, split three ways across
  those jobs
* `cargo audit` (checks Cargo.lock against the RustSec advisory database)
* The vanilla-JS tests of the `human_mapping` site, the showcase viewer and the showcase shim
  (`make test-mapping-site-js`, `test-viewer-js`, `test-showcase-js`)
* `ruff check` and `ruff format --check` over `research/`, `scripts/` and `assets/`, the Python
  unit tests (`make test-python`), the Gentoo `CRATES`/Manifest sync check and `make
  check-versions`
* `make check-third-party-notices`
* The quality gate (`make check-quality`) and the painting gate (`make
  check-painting-attribution`) - see [Quality](src/test/README.md#quality)
* The non-fixture suite on macOS and Windows, with default features - the operating systems the
  release ships binaries for
* `cargo check --locked` on the toolchain `Cargo.toml`'s `rust-version` names
* `cargo package --locked` - the crate builds from its own tarball, as `cargo publish` will see it

All of these checks must pass before a PR is done. Two things run them locally, before GitHub
does - see "Makefile targets" above:

* `make install-hooks` puts the fast subset (fmt, clippy, ruff, the JS tests) on every `git push`,
  so the common mistakes never leave your machine.
* `make ci` runs *all* of the above, driven by parsing `ci.yml` itself so the two cannot drift.
  Minutes, not seconds - it does the release build and full test matrix. What it does not
  reproduce is the runner: it uses your toolchain and OS, where CI gets a clean pinned
  `ubuntu-latest`.
