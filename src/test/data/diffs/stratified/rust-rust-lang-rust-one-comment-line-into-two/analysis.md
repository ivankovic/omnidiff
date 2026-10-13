# Accuracy analysis: rust-rust-lang-rust-one-comment-line-into-two

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 369 nodes, 354 graded, N:M floor 0 visible |
| Painting, Minimal | 12 bytes wrong of 1453: renderer 170, matcher 182 |
| Painting, Full | 12 bytes wrong of 1453: renderer 182, matcher 182 |
| Ground-truth invariant violations | none |

## The change

From rust-lang/rust (`library/compiler-builtins/libm/src/math/generic/fmaximum_num.rs`, commit
7a698be6): one inner doc-comment line, `//! Excluded from our implementation is sNaN handling.`
(line 12), is replaced by two, `//! Note that the IEEE 754-2019 specifies ...` / `//! but we do not
implement this.` (after lines 12-13). `description.md`: "Requires N:M mapping, but a somewhat
weird case of it".

## Mapping

No mismatches, but partly vacuously: the ground truth's tree does not list the changed comment
nodes at all (before `line_comment:11`, after `line_comment:11-12`; 354 of 369 nodes graded). Its
intent is in the paintings: the comment text is deleted and inserted, while the `//! ` marker of
the old line is matched to the markers of both new lines (Full: a 1:2 `Match` on `//` and `!`;
Minimal: the markers left unpainted).

## Painting

Both presets, 12 bytes: the `//! ` markers on line 12 and after lines 12-13. omnidiff deletes the
old comment and inserts both new ones whole, so it paints the markers `Delete`/`Insert` where the
paintings leave them unpainted (Minimal) or moved (Full). A one-to-one mapping could keep the old
line's marker on one new line (8 of the 12 bytes: before line 12 and after line 12), as an
updated comment whose text was replaced; the second new line's marker (4 bytes) needs the 1:2
match. So 8 bytes are the matcher's choice to delete a fully rewritten comment rather than
update it (`matcher`), 4 bytes the N:M floor (`nm-floor`).

The census's renderer share (Minimal 170, Full 182 bytes) is not a renderer fault: rendering the
human mapping, whose tree omits these comment nodes, paints nothing on the three lines, so it
disagrees with the painting on all of their text. `grading-artifact`.

## Verdict

- Primary cause: `matcher`
- Other causes: `nm-floor`, `grading-artifact`
- Where a fix would go: matching a rewritten doc-comment line to its replacement (keep the
  `line_comment` and its marker, replace the `doc_comment` text) where the comment sits in the
  same place; the N:M marker is out of reach for a one-to-one mapping
- Ground truth questionable: no (but the tree leaves the changed comment ungraded, so its mapping
  intent lives only in the paintings)
