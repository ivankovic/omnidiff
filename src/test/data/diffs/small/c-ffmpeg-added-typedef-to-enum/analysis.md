# Accuracy analysis: c-ffmpeg-added-typedef-to-enum

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 2478 nodes, 2478 graded, N:M floor 0 visible |
| Painting, Minimal | 20 bytes wrong of 21144: renderer 0, matcher 20 |
| Painting, Full | 20 bytes wrong of 21144: renderer 0, matcher 20 |
| Ground-truth invariant violations | none |

## The change

FFmpeg `libswscale/ops.h`: `enum SwsCompFlags { ... };` becomes `typedef enum SwsCompFlags { ... }
SwsCompFlags;`, and the struct field `unsigned flags[4];` (line 90) is retyped
`SwsCompFlags flags[4];`; a comment on the next line is re-aligned.

## Mapping

- **`unsigned` -> `SwsCompFlags` (line 90), 1 visible.** The ground truth updates the field's
  type in place. omnidiff deletes `unsigned` and inserts `SwsCompFlags`
  (`APTED("large_flat_subtree")`): the two are different node kinds (`sized_type_specifier` vs
  `type_identifier`), and omnidiff's tree edit distance does not relabel across kinds. `matcher`.

## Painting

- **Delete/Insert -> Update, 20 bytes, both presets** (one painting, `Only one solution`):
  `unsigned` (8) and `SwsCompFlags` (12) on line 90, the mapping above rendered. Rendering the
  human mapping agrees with the painting. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the APTED kind barrier (`large_flat_subtree` call site,
  `apted::common::resolve`): allow an Update between type-specifier kinds in the same type slot
- Ground truth questionable: no
