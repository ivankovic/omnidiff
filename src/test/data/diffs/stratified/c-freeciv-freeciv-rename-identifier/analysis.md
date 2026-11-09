# Accuracy analysis: c-freeciv-freeciv-rename-identifier

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 31642 nodes, 31642 graded, N:M floor 0 visible |
| Painting, Minimal | 16 bytes wrong of 163684: renderer 16, matcher 0 |
| Painting, Full | 0 bytes wrong of 163684: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

freeciv `client/gui-gtk-5.0/cityrep.c`: the constant `PCT_NORMAL_IMPROVEMENT` is renamed
`PCT_IMPROVEMENT` at its two uses (lines 1578 and 1947).

## Painting

- **Minimal, 16 bytes (8 per use), lines 1578 and 1947.** The Minimal painting deletes
  `_NORMAL` (the underscore before the word); omnidiff paints the removed part of the renamed
  identifier as an Update narrowed to `NORMAL_` (the underscore after it): the run covers
  `_NORMAL_`, starting with `_` (None vs Delete) and ending with `_` (Update vs None). Two
  differences in one: the label (Update vs Delete for a removed word of a rename) and which
  underscore goes with the removed word. The same with the human mapping. `renderer`.

Full agrees with the painting; the mapping has no mismatches.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` Minimal Update narrowing of a renamed identifier (label of a
  purely removed word, separator attachment)
- Ground truth questionable: no
