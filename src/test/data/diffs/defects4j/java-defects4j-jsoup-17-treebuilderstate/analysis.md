# Accuracy analysis: java-defects4j-jsoup-17-treebuilderstate

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 35216 nodes, 35216 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 136194: renderer 14, matcher 14 |
| Painting, Full | 0 bytes wrong of 136194: renderer 14, matcher 14 |
| Ground-truth invariant violations | none |

## The change

Line 1451: `nullString = String.valueOf(0x0000);` becomes `String.valueOf('\u0000');`, so the string
holds a NUL character instead of the digit `0`.

## Mapping

- Line 1451, the sole argument `0x0000` -> `'\u0000'`: the ground truth's tree mapping pairs the
  `hex_integer_literal` with the `character_literal` (MatchButNotIdentical). omnidiff
  (`APTED("qualified_name")`) deletes one and inserts the other (different kinds, no common text). 1
  visible.

The ground truth's own painting (`Only one solution`) deletes `0x0000` and inserts `'\u0000'`, which
is omnidiff's reading, so the human file disagrees with itself here. (In
`java-defects4j-codec-4-base64` and `java-defects4j-csv-14-csvformat` a replaced literal of another
kind is an update in both mapping and painting.) The invariant check does not flag it.
`ground-truth`.

## Painting

omnidiff's rendering matches the painting exactly in both presets. Only the HUMAN mapping, rendered,
disagrees (14 bytes): it paints the pair Update where the painting deletes and inserts. That is the
same mapping/painting contradiction in the ground truth.

## Verdict

- Primary cause: `ground-truth`
- Other causes: none
- Where a fix would go: ground truth (Marko's call); nothing in the engine
- Ground truth questionable: yes: tree mapping pairs 0x0000 with '\u0000' (update) but the painting
  deletes and inserts them; not flagged by the invariant check
