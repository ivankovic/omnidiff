# Accuracy analysis: rust-some-to-const-slice

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 5717 nodes, 0 graded, N:M floor 0 visible |
| Painting, Minimal | 2022 bytes wrong of 24036: renderer 2910, matcher 4875 |
| Painting, Full | 2960 bytes wrong of 24036: renderer 2910, matcher 5855 |
| Ground-truth invariant violations | none |

## The change

Handmade, on omnidiff's own language table: the `match ext { "bash" | "sh" =>
Some(Language::ShellScript), ... }` in `language_for_extension` (before lines 74-105) becomes a
`pub const EXTENSIONS: &[(&[&str], Language)]` table (`(&["bash", "sh"],
Language::ShellScript),`, after lines 74-104, on the same rows as the old arms) searched with
`.iter().find(...).map(...)`; new `human_name` and `supported_languages_markdown` functions
follow (after lines 107-179), and two new tests are appended (after lines 295-337).

## Mapping

No mismatches, vacuously: the ground truth's tree has no entries at all (`graded` 0 of 5,717
nodes). For the same reason the census's renderer/matcher split (renderer 2,910, matcher 4,875
bytes) compares omnidiff with the rendering of an empty mapping and says nothing.
`grading-artifact`.

## Painting

One painting, `Minimal`, scored under both presets. Minimal 2,022 wrong bytes:

- `Insert -> None`, 1,154 bytes: the two new tests at the end of the file (after lines 299-337)
  are not painted at all; the painting stops after the first hunk (its last entry ends at after
  line 180). omnidiff inserts them. `ground-truth`.
- `Move -> None`, 526 bytes: the extension strings (`"bash"`, `"sh"`, ...) on every arm row,
  both sides. They moved from `or_pattern`s into array literals but stay on the same rows a few
  columns over, with their neighbours changed; omnidiff paints them `Move`, the painting reads the
  rewrite as in place and leaves them unpainted. `renderer`.
- `Delete`/`Insert -> None`, 324 bytes: on many arm rows (e.g. lines 75 and 78) omnidiff deletes
  and inserts `Language::X` and the separating space and `,` instead of keeping them, where the
  painting leaves `Language::X` unchanged. With no tree to compare against this cannot be graded,
  but it is a mapping choice: the identical scoped identifier is not paired when its arm is
  rebuilt into a tuple. `matcher`.
- 18 bytes: `Some(` painted `Update` where the painting deletes it (9), `find` / `map` in the new
  `language_for_extension` body painted `Update` where the painting inserts them (7), two spaces
  (2). `matcher`.

Full, 2,960 bytes: the same groups (the unpainted tests are 1,522 bytes with their indentation),
plus indentation and punctuation of the inserted lines that Full paints and the Minimal painting
does not. Those extra bytes exist only because Full is held to a Minimal painting.
`grading-artifact`.

## Verdict

- Primary cause: `ground-truth`
- Other causes: `renderer`, `matcher`, `grading-artifact`
- Where a fix would go: ground truth (Marko's call): the tree is empty and the painting stops
  before the second hunk; renderer: Minimal `Move` for nodes re-parented in place
  (`identical_or_move`); matcher: pairing `Language::X` across a match-arm -> tuple rewrite
- Ground truth questionable: yes: the tree has no entries (nothing graded) and the painting does
  not cover the two tests added at after lines 295-337
