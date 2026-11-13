# OmniDiff

[![CI](https://github.com/ivankovic/omnidiff/actions/workflows/ci.yml/badge.svg)](https://github.com/ivankovic/omnidiff/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/ivankovic/omnidiff)](https://github.com/ivankovic/omnidiff/releases/latest)
[![docs.rs](https://docs.rs/omnidiff/badge.svg)](https://docs.rs/omnidiff)
[![coverage](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/ivankovic/omnidiff/main/research/data/coverage/badge.json)](src/test/README.md#coverage)
[![License: AGPL v3](https://img.shields.io/badge/license-AGPL--3.0--only-blue)](LICENSE)

Fast, robust, accurate content-aware diffing.

Supports code in 26 languages, images, fonts, PDFs, archives, and more.

Most diffs under 100 milliseconds on a T14s laptop from 2023.

**[See it in the browser](https://ivankovic.github.io/omnidiff/showcase/)**: real changes to
code, PDFs, fonts, archives and pictures on one page, each shown as Unix `diff` reports it and as
OmniDiff does, recorded from the command-line tool, in a theme of your choice. The animation below
is taken from that page.

![An animation of one Python refactoring painted two ways. A vertical bar sweeps left to right and
back across a two-pane diff. On one side of the bar, Unix diff marks whole lines as deleted and
inserted; on the other, OmniDiff paints only the parts that changed - `sum(numbers)` and
`len(numbers)` rather than the whole assignment, and `numbers` shown as moved rather than
rewritten.](/assets/diff-vs-omnidiff.gif)

# Quick start

```
cargo install --locked omnidiff          # or: brew install ivankovic/omnidiff/omnidiff
omnidiff BEFORE AFTER                    # the interactive viewer; press ? for the keys
omnidiff git configure                   # then: git difftool
```

There are also prebuilt binaries, a signed apt repository, a Nix flake, and extensions for
[VS Code](https://github.com/ivankovic/omnidiff-vscode) and
[Neovim](https://github.com/ivankovic/omnidiff.nvim).
[docs/README.md](docs/README.md#installation) has every way to install.

# Using OmniDiff

- **Interactive viewer:** `omnidiff BEFORE AFTER`, or `omnidiff --review` to pick from a
  repository's unstaged files, staged files and recent commits.
- **Plain text:** `omnidiff --headless BEFORE AFTER`, stacked or side by side, for scripts, CI and
  pagers. It is chosen automatically when stdout is not a terminal.
- **JSON:** `omnidiff --mode json BEFORE AFTER`, one object described by a
  [JSON Schema](src/tui/json_output.schema.json), for editor integrations.
- **git:** `omnidiff git configure` sets it up as `git difftool`, and optionally for `git diff`
  and `git log -p`.
- **jj:** `omnidiff jj configure` sets it up as a `jj diff` formatter.
- **lazygit:** as the diff renderer of lazygit's main view.

[docs/README.md](docs/README.md) describes each of these, and lists the
[supported languages and formats](docs/README.md#supported-content). A file OmniDiff has no
grammar for is diffed as plain text, line by line, so nothing is refused.

# Deeper dives

| | |
|---|---|
| [Using OmniDiff](docs/README.md) | Installation, the viewer, headless and JSON output, git, jj, lazygit and editor integration, supported content |
| [Testing](src/test/README.md) | The corpus of real changes and its human ground truth; the speed, robustness and accuracy goals and where OmniDiff stands against each; how to run the tests |
| [What real-world changes look like](research/data/corpus_stats/README.md) | What commits in 7,444 repositories change, and how large the files and the edits are |
| [Comparison with other tools](research/data/comparison/README.md) | Unix `diff`, `git diff`, BDiff, `nvim -d`, GumTree, difftastic, diffsitter and srcDiff on the same changes, and OmniDiff against an external oracle |
| [Contributing](CONTRIBUTING.md) | Technology, code quality, project structure, Makefile targets, CI. `AGENTS.md` has additional AI-agent-specific conventions |
| [Packaging](packaging/README.md) | Package-manager recipes and the release checklist |
| [Research](research/README.md) | The corpus, the measurements over it, the analysis scripts and the papers |

# AI policy

This project uses substantial AI assistance, currently Claude Code. Most commits disclose this
with a `Co-Authored-By` trailer and a link to the session that produced them. This project does not
hide that fact. This project does not treat AI assistance as a lesser way to write software.

Contributions are not accepted at this time, to keep the development speed high (see
[CONTRIBUTING.md](CONTRIBUTING.md)). When that changes, AI-assisted contributions will be as
welcome as any other, disclosed the same way: whoever submits the work is responsible for
understanding it and standing behind it.

# License

Copyright (C) 2026 Marko Ivankovic

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published
by the Free Software Foundation, version 3 of the License.

See the LICENSE file for the full text of the License.

## Cannot use AGPL software?

A commercial license is available as a monthly subscription through
[GitHub Sponsors](https://github.com/sponsors/ivankovic). It covers internal use of omnidiff
by your organisation without the source-disclosure obligations of the AGPL. The terms are in
[LICENSE-COMMERCIAL](LICENSE-COMMERCIAL). Pick the tier that names the commercial license as a
benefit. For invoicing or other arrangements, contact me at
[marko@ivankovic.me](mailto:marko@ivankovic.me).
