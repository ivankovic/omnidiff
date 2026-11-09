# Accuracy analysis: java-defects4j-codec-17-stringutils

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 2163 nodes, 2163 graded, N:M floor 0 visible |
| Painting, Minimal | 16 bytes wrong of 38935: renderer 0, matcher 16 |
| Painting, Full | 16 bytes wrong of 38935: renderer 0, matcher 16 |
| Ground-truth invariant violations | none |

## The change

Line 339: `return new String(bytes, Charsets.ISO_8859_1);` becomes `return newString(bytes,
Charsets.ISO_8859_1);`, calling the null-safe helper instead of the constructor.

## Mapping

- Line 339, `String` (the `type_identifier` of the `object_creation_expression`) and `newString`
  (the `identifier` of the new `method_invocation`): the ground truth deletes the first and inserts
  the second. omnidiff (`APTED("qualified_name")`) maps them to each other as MatchButNotIdentical,
  across node kinds, because they sit at the same place in two otherwise identical argument lists. 2
  visible. The constructor call and the helper call are different constructs and share only the
  substring `String`; the ground truth is reasonable. `matcher`.

## Painting

Matcher-caused (none with the human mapping), the same in both presets, 16 bytes on line 339: `None
-> Delete` on ` String` (7 bytes, before) and `Update -> Insert` on `newString` (9 bytes, after).
omnidiff paints `new` deleted but renders `String` -> `newString` as an Update. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED fallback (qualified_name): mapping a type_identifier onto an
  identifier of a different construct
- Ground truth questionable: no
