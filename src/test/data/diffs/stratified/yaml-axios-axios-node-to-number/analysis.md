# Accuracy analysis: yaml-axios-axios-node-to-number

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 238 nodes, 238 graded, N:M floor 0 visible |
| Painting, Minimal | 6 bytes wrong of 1072: renderer 0, matcher 6 |
| Painting, Full | 6 bytes wrong of 1072: renderer 0, matcher 6 |
| Ground-truth invariant violations | none |

## The change

In `.travis.yml` the last `node_js` version in the build matrix changes from `node` to `14`
(line 7).

## Mapping

- Line 7, 1 visible. The ground truth updates the list item's value `node` (a `string_scalar`)
  to `14` (an `integer_scalar`), MatchButNotIdentical. omnidiff deletes `node` and inserts `14`,
  reason `APTED("qualified_name")`: the two leaves have different node kinds, so APTED did not
  pair them although they sit at the same place in the same list item. The value of one list
  entry changed; the ground truth's update is what a reader expects. `matcher`.

## Painting

The single painting (`Only one solution`) is scored under both presets; both give the same 6
bytes, all matcher-caused (none with the human mapping): `node` painted Delete and `14` painted
Insert where the painting has one Update.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED leaf rename cost in `solve_syntax_aware_matching` (`qualified_name`):
  YAML scalar kinds (`string_scalar`, `integer_scalar`, ...) should be renameable into each other
  when the parent is matched.
- Ground truth questionable: no
