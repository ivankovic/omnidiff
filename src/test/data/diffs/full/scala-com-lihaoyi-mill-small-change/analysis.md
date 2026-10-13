# Accuracy analysis: scala-com-lihaoyi-mill-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 4 (4 visible) of 1622 nodes, 1622 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `RunnerState`, the case-class parameter `staticBuildOverrides: Map[String, ujson.Value]` becomes
`buildOverrideFiles: Map[java.nio.file.Path, String]` (line 72 on both sides): the parameter is
renamed and the map's key and value types change, with `String` moving from key to value.

## Mapping

All 4 visible mismatches are on line 72, reason `APTED("qualified_name")`.

- The ground truth keeps `String` (before column 33) and its `,` Identical, moved to the value
  position, updates the type name `Value` -> `Path`, and maps `ujson` into a 1:3 group with the new
  qualifier. omnidiff deletes the key `String` and the `,` (2 visible) and maps `Value` (a
  `type_identifier`, before column 47) to `nio` (an `identifier` in the qualifier, after column 36)
  as MatchButNotIdentical (2 visible). Pairing the last segment of `ujson.Value` with a middle
  segment of `java.nio.file.Path`, across node kinds, is the same pattern seen in other mill
  fixtures (`Value` -> `core`). The ground truth's reading follows the text. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED in `solve_syntax_aware_matching` (`qualified_name`): prefer
  `type_identifier` -> `type_identifier` over cross-kind pairs inside `stable_type_identifier`,
  and let an identical sibling (`String`) be kept across a reorder of type arguments.
- Ground truth questionable: no
