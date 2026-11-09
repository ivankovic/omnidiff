# Accuracy analysis: javascript-microsoft-typescript-concat-to-template

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 108 nodes, 108 graded, N:M floor 0 visible |
| Painting, Minimal | 12 bytes wrong of 562: renderer 0, matcher 12 |
| Painting, Full | 12 bytes wrong of 562: renderer 6, matcher 12 |
| Ground-truth invariant violations | none |

## The change

TypeScript baseline `templateStringWithEmbeddedConditional.js`, line 7:
`var x = "abc".concat(true ? false : " ", "def");` becomes
`` var x = `abc${true ? false : " "}def`; ``.

## Mapping

2 visible, the `string_fragment`s `abc` (col 10) and `def` (col 43) on line 7. The ground truth
maps them Identical into the new `template_string`. omnidiff deletes them (and inserts the after
fragments), `APTED("qualified_name")`: the `call_expression` / `member_expression` / `string`
parents are gone and the `template_string` is new, and the APTED pass does not pair leaves whose
parents are both unmatched and of a different kind. `matcher`.

## Painting

Matcher-caused (12 bytes Minimal, 12 Full; none of it in the human-mapping rendering in
Minimal): `abc` and `def` on line 7, both sides, are Delete / Insert in omnidiff's rendering.
Minimal leaves them unpainted (unchanged text), Full paints them Move. `matcher`.

Renderer residual (Full, 6 bytes): rendered from the human mapping, `abc` (same row and column on
both sides) stays unpainted where the Full painting marks it Move for its new parent; `def`, whose
column changed, is painted Move as in the painting. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: `APTED("qualified_name")` (carry identical leaves across a replaced
  parent); Full move painting for reparented in-place text
- Ground truth questionable: no
