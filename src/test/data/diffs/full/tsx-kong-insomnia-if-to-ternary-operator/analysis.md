# Accuracy analysis: tsx-kong-insomnia-if-to-ternary-operator

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 5 (5 visible) of 13270 nodes, 13270 graded, N:M floor 0 visible |
| Painting | no painting on file |
| Ground-truth invariant violations | none |

## The change

Lint-driven modernisation of `tag-editor.tsx`: `getAttribute('data-…')` becomes `dataset.…` (lines
181, 185), the globals `parseInt`/`parseFloat` become `Number.parseInt`/`Number.parseFloat` (lines
182, 190), and an `if (error.endsWith(linkText)) { previewElement = (…); } else { previewElement =
<textarea …/>; }` (before lines 259-276) becomes a single assignment of a ternary (after lines
259-274).

## Mapping

- `parseInt` / `parseFloat` gaining a `Number.` receiver, lines 182 and 190, 4 visible: the ground
  truth maps the old callee identifier to the same name as the new `property_identifier`
  (`parseInt` -> `Number.parseInt`) and inserts `Number`. omnidiff maps the old identifier to
  `Number` (an Update) and inserts the property, reason `APTED("qualified_name")`. This is a cost
  tie under `UnitCostModel::ren` (src/diff/apted/common.rs): `identifier` -> `property_identifier`
  is a cross-kind pair, which `update_allowed_from_masks` allows for identifier-like kinds but
  prices at `COST_UPDATE` (1) even when the text is identical. So keeping the name costs the same
  as renaming `parseInt` into `Number`, and APTED's tie-break takes the leftmost leaf. `matcher`.
- Before line 275, the `;` of the else branch's `previewElement = <textarea …/>;`, 1 visible: two
  assignments (if branch and else branch) merge into one `previewElement = cond ? (…) : (…);`.
  The ground truth keeps the else branch's `;` as the new statement's `;`; omnidiff deletes it
  (reason `APTED("qualified_name")`). Which of the two old statements' terminators survives in a
  merge like this is a judgement call. `ambiguous`.

## Verdict

- Primary cause: `matcher`
- Other causes: `ambiguous`
- Where a fix would go: `UnitCostModel::ren` (src/diff/apted/common.rs), which prices an
  identical-text `identifier` -> `property_identifier` pair like any rename, so it ties with
  renaming into the new receiver
- Ground truth questionable: no
