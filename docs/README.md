# Using OmniDiff

How to install OmniDiff, how to run it on its own and under git, jj, lazygit and an editor, and
what content it diffs. The [project README](../README.md) has the overview.

- [Installation](#installation)
- [The interactive TUI](#the-interactive-tui)
- [Headless / batch mode](#headless--batch-mode)
- [JSON output](#json-output)
- [Git integration](#git-integration)
- [lazygit integration](#lazygit-integration)
- [Jujutsu (jj) integration](#jujutsu-jj-integration)
- [Supported content](#supported-content)

## Installation

### From crates.io

```
cargo install --locked omnidiff
```

This command builds OmniDiff from source, with the dependency versions it was tested with. You
need a C compiler on `PATH` and a Rust toolchain, rustc 1.92 or later. The build compiles every
tree-sitter grammar from C. The first `cargo install` takes a few minutes, because of this and the
`lto = "fat"` release profile.

`cargo install` does not install shell completions or the man page. Generate them from the
binary:

```
omnidiff util completions bash   # or zsh, fish
omnidiff util man
```

The packages below install both.

### Prebuilt binaries

Prebuilt binaries for Linux, macOS (Intel and Apple Silicon), and Windows are attached to every
[GitHub release](https://github.com/ivankovic/omnidiff/releases/latest).

### Homebrew

macOS and Linux:

```
brew install ivankovic/omnidiff/omnidiff
```

### Debian and Ubuntu

`.deb` packages for amd64 and arm64 are available in a signed apt repository:

```sh
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL https://ivankovic.github.io/omnidiff/apt/omnidiff-archive-keyring.gpg \
  | sudo tee /etc/apt/keyrings/omnidiff-archive-keyring.gpg > /dev/null
echo "deb [signed-by=/etc/apt/keyrings/omnidiff-archive-keyring.gpg] \
https://ivankovic.github.io/omnidiff/apt stable main" \
  | sudo tee /etc/apt/sources.list.d/omnidiff.list > /dev/null
sudo apt update && sudo apt install omnidiff
```

### Nix and NixOS

On NixOS, or anywhere with Nix installed:

```
nix run github:ivankovic/omnidiff
```

If flakes are not enabled in your Nix configuration, add
`--extra-experimental-features 'nix-command flakes'` after `nix`.

### Arch and Gentoo

Recipes for Arch and Gentoo live in [`packaging/`](../packaging/): the PKGBUILD builds locally
with `makepkg -si`, and the Gentoo ebuild is ready for an overlay. Neither is submitted to its
distribution: the AUR is closed to new submissions.

### Editor integration

* **VS Code** - [omnidiff-vscode](https://github.com/ivankovic/omnidiff-vscode). Search for
  **OmniDiff** in the Extensions view, or `code --install-extension ivankovic.omnidiff`. Also on
  [Open VSX](https://open-vsx.org/extension/ivankovic/omnidiff) for VSCodium, Cursor and Windsurf.
  VS Code does not yet let an extension replace its built-in diff view. The API is implemented
  upstream but unreleased; the extension will adopt it when it ships.
* **Neovim** - [omnidiff.nvim](https://github.com/ivankovic/omnidiff.nvim).

## The interactive TUI

```
omnidiff                  # open an empty viewer; press o to pick each file
omnidiff BEFORE AFTER     # open directly into the diff of two files
omnidiff --review         # open on the git review picker
```

Press `?` in the viewer for the full list of keybindings. The ones to know first:

| Key | Does |
|---|---|
| `n` / `p` | Jump to the next or previous change |
| `Enter` | Jump to the counterpart of the range under the cursor |
| `/` | Search the focused panel |
| `o` | Open a file in the active panel |
| `G` | Review git changes: unstaged files, staged files and recent commits |
| `M` | Settings, on the Highlighting tab |
| `c` | Settings, on the Colors tab: themes and every diff color |
| `v` | Cycle the panel layout: auto, dual, single |
| `q` | Quit |

### Highlighting: Minimal and Full

OmniDiff has two highlighting presets, and both are faithful readings of the same diff. Minimal
paints only the ranges that carry meaning: standalone brackets and separators, leading whitespace,
and moves that only reindent go unpainted. Full paints them. In the settings menu (`M`), `1` and
`2` pick the presets and each option has its own checkbox; the choice is saved. `--minimal` and
`--full` apply a preset to one run without saving it, and `--whole-updates` highlights both sides of
an updated pair whole instead of only the part that differs.

### Colors

The TUI uses 24-bit color when the terminal advertises it with `COLORTERM=truecolor`, and the
nearest 256 colors otherwise (macOS Terminal.app, or most terminals over ssh, which does not
forward `COLORTERM`). If your terminal supports 24-bit color but does not advertise it, run
`export COLORTERM=truecolor`.

## In a browser

For reviewing changes in a browser, see [codereview](https://github.com/ivankovic/codereview).

**Note: codereview is at v0.0.0; use it at your own risk.**

## Headless / batch mode

`omnidiff --headless BEFORE AFTER`, or its synonym `--batch`, prints the diff as plain text, with
optional color, instead of opening the TUI. Use this for scripts or CI. Headless mode also starts
automatically whenever stdout is not a terminal, for example when piped into `less` or redirected to
a file. Because of this, `omnidiff BEFORE AFTER | less` works without the flag.

The output is stacked by default: the before side, then the after side. `--layout side-by-side`
puts the two sides in two columns, the lines OmniDiff matched on one row and long lines wrapped;
`--layout auto` does so only when the columns are wide enough for nearly every shown line. The
width is `--width N`, else `$COLUMNS`, else the terminal's.

OmniDiff collapses long runs of unchanged lines. It keeps 3 lines of context on each side of a
change (override with `--context N`), the same convention as `diff -u`. OmniDiff also prefixes each
hunk with the nearest enclosing function, class, or struct line, when that line is not otherwise
visible. This shows the location of a change deep inside a large file.

Colors are on by default; pass `--color never`, or set `NO_COLOR=1`, to disable ANSI colors, for
example when you redirect output to a file - `--color always` forces them even under `NO_COLOR`.

OmniDiff exits `0` on success and `2` on error. For scripting, pass `--exit-code` to additionally
get `1` when the files differ. This follows the `diff(1)` convention. `--exit-code` is opt-in rather
than the default for the same reason `git diff` exits `0` even when files differ: OmniDiff's usual
non-interactive callers are version control systems driving it as a display tool, and they read a
non-zero exit as "the tool failed" - `jj` warns on every file, and `git difftool` with
`difftool.trustExitCode=true` aborts the whole diff. (The 7-argument `GIT_EXTERNAL_DIFF` form stays
at `0` even with `--exit-code`, since git treats a non-zero exit there as fatal.)

## JSON output

`omnidiff --mode json BEFORE AFTER` prints the diff as one JSON object. Its fields are described
in [`src/tui/json_output.schema.json`](../src/tui/json_output.schema.json), a JSON Schema.

## Git integration

OmniDiff is a `git difftool` backend. Run the interactive setup wizard, which asks
whether to configure it globally or for the current repository only:

```
omnidiff git configure
```

### Configure by hand

```
git config difftool.omnidiff.cmd 'omnidiff -- "$LOCAL" "$REMOTE"'
git difftool --tool=omnidiff
```

Run `git config diff.tool omnidiff` to make plain `git difftool` use OmniDiff by default, without
needing `--tool`. If you do not want git to ask "view diff ... [Y/n]?" before every file, run `git
config difftool.prompt false`.

### Usage

**`git difftool` opens the interactive TUI. `git diff` and `git log -p` never do.** `git diff`
pipes its output through git's pager, and a full-screen TUI cannot draw onto a pipe, so OmniDiff
prints plain text there (see "Headless / batch mode" above). If you want the interactive viewer from git, use `git difftool`,
not `git diff`. If `git difftool` still doesn't open interactively over SSH, reconnect with
`ssh -t` — the session needs an allocated pseudo-terminal; tmux panes always have one.

OmniDiff also works directly with `git diff` and `git log -p`, through `GIT_EXTERNAL_DIFF`. This
path needs no `difftool` config:

```
GIT_EXTERNAL_DIFF=omnidiff git diff
```

Pictures, archives, fonts and the rest of the [supported content](#supported-content) get a short
report of what changed in them. Any other binary file gets a one-line `Binary file <path> differs`
notice instead of a diff, the same stand-in git and `diff(1)` print for them. Neither ever blocks
the rest of a `git diff`: an external diff that exits non-zero makes git abandon the *entire* run,
so OmniDiff reports an unshowable file as a successful diff of nothing rather than as a failure.

## lazygit integration

lazygit 0.66 and later shows a file's diff through a configured renderer. In lazygit's
`config.yml`:

```yaml
git:
  diffRenderers:
    - type: extDiff
      command: "omnidiff --context {{diffContext}} --width {{width}} --layout auto"
```

`{{width}}` is the width of lazygit's main view; with it, `--layout auto` shows two columns when
they fit and stacks the sides otherwise. Staging single lines from the focused diff keeps
OmniDiff's rendering. `make test-lazygit` drives a real lazygit with OmniDiff as its renderer.

## Jujutsu (jj) integration

jj does not read git's `difftool`/`diff.external` settings, even in a colocated repo, so it needs
its own configuration. Run the setup wizard:

```
omnidiff jj configure
```

### Configure by hand

```
jj config set --user merge-tools.omnidiff.program omnidiff
jj config set --user merge-tools.omnidiff.diff-args '["--","$left","$right"]'
jj config set --user merge-tools.omnidiff.diff-invocation-mode file-by-file
```

That registers `jj diff --tool omnidiff`. To make it the default for plain `jj diff` as well:

```
jj config set --user ui.diff-formatter omnidiff
```

Use `--repo` in place of `--user` to configure the current repository only.

**`diff-invocation-mode = "file-by-file"` is required.** jj's default hands a diff tool two
*directory* trees; OmniDiff diffs two files, so without this setting every invocation fails. With
it, jj passes one changed file pair at a time, keeping each file's real path and extension, so
language detection works exactly as it does under git.

### Usage

`jj diff` runs its formatter under a pager, so OmniDiff renders in its non-interactive text mode
there - the same output `git diff` gets. jj has no equivalent of `git difftool`'s interactive
per-file viewer (its terminal-attached hook, `ui.diff-editor`, is for `jj diffedit`/`jj split`,
which edit the right-hand side and read it back - not something a read-only viewer should claim to
do), so for the full-screen TUI on a jj repo, run `omnidiff BEFORE AFTER` directly.

## Supported content

The language is detected from the file extension. A file with an unknown extension is diffed as
plain text, line by line, so nothing is refused.

<!-- languages:start -->
26 languages are parsed with a tree-sitter grammar and diffed structurally:

| Language | File extensions |
|---|---|
| C | `.c`, `.h` |
| C# | `.cs` |
| C++ | `.cc`, `.cpp`, `.cxx`, `.hpp`, `.hh`, `.hxx` |
| CSS | `.css`, `.scss` |
| Go | `.go` |
| HTML | `.html`, `.htm` |
| Java | `.java` |
| JavaScript | `.js`, `.mjs`, `.cjs`, `.jsx` |
| JSON | `.json` |
| Kotlin | `.kt` |
| Lua | `.lua` |
| Perl | `.pl`, `.pm` |
| PHP | `.php` |
| Python | `.py`, `.pyi`, `.pyw` |
| R | `.r` |
| Ruby | `.rb` |
| Rust | `.rs` |
| Scala | `.scala` |
| Shell (Bash) | `.bash`, `.sh` |
| SQL | `.sql` |
| Swift | `.swift` |
| TSX | `.tsx` |
| TypeScript | `.ts`, `.mts`, `.cts` |
| Vimscript | `.vim` |
| XML | `.xml`, `.xht`, `.xhtml`, `.svg`, `.glif`, `.plist`, `.ui`, `.qrc`, `.glade`, `.xib`, `.storyboard`, `.xaml`, `.vcxproj`, `.filters`, `.csproj`, `.fsproj`, `.vbproj`, `.props`, `.targets`, `.nuspec`, `.resx`, `.wxs`, `.iml`, `.manifest`, `.config`, `.policy`, `.xsd`, `.xsl`, `.xslt`, `.xlf`, `.xliff`, `.kml`, `.gpx`, `.rss`, `.atom`, `.graphml`, `.dae` |
| YAML | `.yaml`, `.yml` |

Recognised by extension but diffed as plain text, since no grammar is compiled in: Bazel (`.bazel`), Dart (`.dart`), Emacs Lisp (`.el`), Markdown (`.md`, `.markdown`), Protocol Buffers (`.proto`).
<!-- languages:end -->

Files that are not text are recognised by their bytes, never by their names, and diffed by what
they hold:

| Content | Formats | Diffed |
|---|---|---|
| Pictures | PNG, JPEG, GIF, WebP, BMP, ICO, TIFF | pixel by pixel, as the eye compares them: how much changed and where; animations frame by frame |
| Archives | zip (and so jar, Office documents, EPUB), tar, gzip, xz, bzip2 | file by file, each by what it is, archives inside archives included; Java class files as a listing of their fields and methods |
| Fonts | TrueType, OpenType, WOFF, WOFF2, EOT | glyph by glyph, each drawn, and the font's names and metrics |
| Cursors | Windows `.cur` and `.ani`, X cursors, Hyprland `.hlc` | size by size and frame by frame |
| Message catalogs | gettext `.mo`, Qt `.qm` | message by message |
| Documents | PDF | page by page, each page drawn |
| Text in other encodings | UTF-16 and UTF-32 with a byte order mark | as text, like any other file |

Only the members that changed are opened, so a font with two edited glyphs shows two glyphs. Each
change is also named by how much of it a reader would see: invisible (only the bytes differ),
imperceptible, artifacts (what compression or resampling leaves behind), edited, redrawn or
replaced - for text, whitespace, formatting and rewritten stand in for the second, third and
fifth.

The TUI shows pictures as pictures: in the terminal's own graphics where it speaks the kitty,
sixel or iTerm2 protocol, and in Unicode half blocks everywhere else. `b` switches to half blocks
and back, and `OMNIDIFF_GRAPHICS=halfblocks` (or `kitty`, `sixel`, `iterm2`) overrides the
detection; inside tmux, the terminal in front of the pane decides. `omnidiff util graphics` prints
which one the TUI would use in this terminal, and why. `t` cycles a picture pair's views: side by
side, changed areas, difference, blend, swipe. An archive, a font or any other
container lists its changed members beside the one selected, `g` shows every changed glyph or
picture at once, and `Enter` opens a text member in the full diff view.

Anything else binary gets the one-line `Binary file <path> differs` notice.
