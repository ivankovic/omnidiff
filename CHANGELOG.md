# Changelog

All notable changes to OmniDiff (named CodeDiff up to v0.1.1). The format follows [Keep a Changelog](https://keepachangelog.com/),
and the version numbers follow [Semantic Versioning](https://semver.org/) as far as a 0.x
release does: a minor bump may change the JSON output or the library API, a patch bump does not.

## [Unreleased]

### Added

- Pictures (PNG, JPEG, GIF, WebP, BMP, ICO, TIFF) are diffed instead of reported as "Binary file
  differs": what each side is (format, size, color type), how much of it changed and where, as
  rectangles of changed pixels. Pixels compare perceptually, so re-encoding noise is not a change.
  Headless and `git diff` print this as a short report and `--mode json` adds a `picture` object.
  The TUI shows the two pictures with the changed regions outlined, and `t` cycles to a
  difference, blend and swipe view; it draws real pixels where the terminal speaks the kitty,
  sixel or iTerm2 graphics protocol and Unicode half blocks everywhere else.
- Animated GIF, PNG and WebP pictures are compared frame by frame, the frames aligned the way
  lines are: the report says which frames changed, which were added or removed, and whether the
  same frames now show for different times. In the TUI, `,` and `.` step through the frames and
  space plays the animation.
- Inside tmux, the TUI draws pictures with the graphics protocol of the terminal in front of the
  pane, as tmux knows it, rather than whichever attached client answered the query: a kitty on a
  desk no longer makes a phone's terminal attached to the same session show crossed-out boxes.
  `b` switches every picture to half blocks and back, and `OMNIDIFF_GRAPHICS=halfblocks` (or
  `kitty`, `sixel`, `iterm2`) overrides the detection.
- Archives, fonts, cursor themes, message catalogs and PDFs are diffed by what they hold, member
  by member: a zip's or tar's files (jars, Office documents and EPUBs are zips; gzip, xz and
  bzip2 streams are opened too), a font's glyphs and names (TrueType, OpenType, WOFF, WOFF2,
  EOT), a cursor's sizes and frames (Windows `.cur` and `.ani`, X cursors, Hyprland `.hlc`), a
  gettext `.mo` or Qt `.qm` catalog's messages, a PDF's pages. Members match by name and only
  the changed ones are opened: each is diffed by what it is - a picture as a picture, text by
  lines, a nested archive as an archive. The report lists the members that changed, were added or
  were removed; `--mode json` adds a `content` object, which a picture pair carries too. In the
  TUI the members are listed beside the selected one, `a` shows the unchanged ones as well, `g`
  shows every changed glyph or picture at once, and `Enter` opens an archive inside an archive.
  A text member is diffed by OmniDiff in its own language, its long lines wrapped and cut to the
  changes (`J`/`K` step through them), and `Enter` opens it in the full diff view; a Java class
  file in a jar is diffed as a listing of its fields, its methods and what each method calls,
  reads and loads.
- What happened to a picture or a member is said in one of six levels, by how hard it is to see
  and what the after side is: invisible (only bytes differ), imperceptible (seen only once
  highlighted), artifacts (what compression, resampling or a formatter leaves behind), edited,
  redrawn (the same thing made anew) and replaced; with tags for a picture resized, cropped,
  retimed or given frames, and for a container given members. Pictures of one aspect ratio and
  different sizes are compared at the smaller size; `--mode json` reports, per comparison, the
  pixels that differ at all and the clearly changed ones beside those that changed.
- Text in UTF-16 or UTF-32 (announced by a byte order mark) is read as text and diffed like any
  other file instead of reported as binary; `--mode json` names its `encoding`.

### Changed

- XML formats with an extension of their own are diffed with the XML grammar instead of line by
  line: `.svg`, `.plist`, Qt `.ui`/`.qrc`, MSBuild `.vcxproj`/`.csproj`/`.props`/`.targets`, `.xib`,
  `.storyboard`, `.xaml`, `.resx`, `.xsd`/`.xsl`, `.xlf`, `.kml`, `.gpx` and more (see the README's
  language list). A file under one of these names whose content is not markup is still diffed as
  plain text.
- Building OmniDiff needs Rust 1.92 or later (was 1.88): the picture view's graphics library
  needs 1.90, and the PDF renderer 1.92.
- The TUI shows the two files side by side from 200 terminal columns (was 220); narrower
  terminals still show one panel at a time.

## [0.2.0] - 2026-10-01

**CodeDiff is now OmniDiff.** The old name is a registered trademark, so everything was renamed.

### Changed

- **Renamed from CodeDiff to OmniDiff.** The crate, binary, packages, editor extensions and
  repositories are all `omnidiff` now: install `omnidiff` (`cargo install --locked omnidiff`, the
  `omnidiff` deb from `https://ivankovic.github.io/omnidiff/apt`, `brew install
  ivankovic/omnidiff/omnidiff`, the `ivankovic.omnidiff` VS Code extension, `omnidiff.nvim`),
  then rerun `omnidiff git configure` and `omnidiff jj configure`, since a configuration that
  names `codediff` points at a binary that no longer updates. The apt repository moved with the
  GitHub Pages site: replace `ivankovic.github.io/codediff/apt` and the
  `codediff-archive-keyring.gpg` key file in your apt sources with the `omnidiff` ones (see the
  README). The `codediff` crate and the `codediff` packages stay available for now and are
  retired in a later release.
- The config moved to `.omnidiff.toml`, `$OMNIDIFF_CONFIG` and `~/.config/omnidiff/config.toml`.
  For this release the old names are still read, and `~/.config/codediff/config.toml` is moved to
  the new place the first time it is needed.
- Code copied to several places is shown as such: when a statement or declaration of some size
  appears once before and twice after (or the reverse) in two branches of one `if`, every copy
  paints as a move of the original instead of one being matched and the rest inserted.
- A comment of three or more words that moves inside a function, class or other construct that
  survives the change is shown as moved, instead of deleted and re-inserted.
- A run of comments or small statements that changes length is aligned by similarity, so the
  entries that stayed are matched instead of the whole run being deleted and re-inserted.
- A leaf renamed across two different constructs (one identifier of a replaced expression reused
  in the new one) is shown as deleted and inserted rather than as an in-place update.
- Library: `ASTDiff` records N:M groups next to its one-partner node maps (`add_group`), and its
  node maps can no longer disagree about a pair (`is_valid` checks it).

## [0.1.1] - 2026-09-27

Responding to initial user reports for v0.1.0.

Note the removal of "--web" and sending users to CodeReview instead.

### Changed

- Tabs display at tab stops, in the TUI and in headless output, instead of as one space.
- The TUI uses the nearest 256 colors when the terminal does not advertise 24-bit color (macOS
  Terminal.app).
- `--minimal`, `--full`, `--whole-updates` and `--paint-reindent-moves` apply in the TUI too, for
  that run only.
- `--review` rejects a BEFORE/AFTER pair and `--mode`, and `--headless` rejects `--mode`, instead
  of ignoring one flag.
- `codediff git configure` writes the resolved path of the running binary to `diff.external`.
- `change N/M` counts changes in the order `n` walks them.
- The render-options badge names what differs from `--full`.
- Panel titles name languages as people write them (C++, Lua, Markdown). The JSON `language`
  values are unchanged.
- Library: the `diff::solve_*` pass modules are crate-private, `ASTDiff::is_valid` has no `after`
  parameter, and `NodeCache` takes a lifetime (see Fixed). **Breaking** for library users, against
  the policy above; the library API was never meant to be relied on (see the crate docs).

### Fixed

- The TUI cursor lands on the right column on lines with non-ASCII text.
- Ctrl-C quits from every screen, and Ctrl-E scrolls instead of opening `$EDITOR`.
- The theme dialog opens on the syntax theme in use, and accepting a preset keeps the saved Custom
  palette.
- Home and End move the cursor, not only the view.
- `$VISUAL`/`$EDITOR` may carry arguments (`code -w`, `emacsclient -t`).
- Omitting `--whole-updates` no longer switches a saved "Whole-pair updates" setting off.
- The TUI exits when its input stream closes.
- A block whose children were all deleted no longer takes its whole subtree with it: code matched
  inside it (a body moved from an `if` into an `unless`) is shown as matched, not deleted and
  inserted.
- Library: `NodeCache` borrows the `Code` it was built from, so a cache that outlives its `Code`
  no longer compiles; before, safe code could read freed memory through it.

### Removed

- The browser front end (`codediff-web`) and the `web` Cargo feature. **Breaking** for anyone who
  installed with `cargo install codediff --features web`; for reviewing changes in a browser, use
  [codereview](https://github.com/ivankovic/codereview). The GitHub Pages showcase is unaffected.

## [0.1.0] - 2026-09-25

The first release ready for users!

Ready for:

-  Daily usage as a `git difftool` tool!
-  Daily usage in an IDE!
-  Integration into batch pipelines and LLM agents!

Things that you can do, but expect changes:

-  Integration as a library into other products. The API is not fixed and might change.

### Metrics:

-  Speed: p50 7.6ms, p90 78.7ms, p99 347ms, slowest 1,355ms. 100ms is the 92.7th percentile
-  Robust: 99.95%
-  Accurate: 70.6% perfect, 88.6% <= 1% off

## [0.0.14] and earlier

Pre-releases; see the [GitHub releases](https://github.com/ivankovic/codediff/releases).
