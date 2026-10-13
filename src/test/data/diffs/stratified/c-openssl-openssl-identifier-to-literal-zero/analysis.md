# Accuracy analysis: c-openssl-openssl-identifier-to-literal-zero

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 52318 nodes, 52318 graded, N:M floor 0 visible |
| Painting, Minimal | 16 bytes wrong of 271718: renderer 0, matcher 16 |
| Painting, Full | 16 bytes wrong of 271718: renderer 0, matcher 16 |
| Ground-truth invariant violations | none |

One painting (`Only one solution`), scored under both presets.

## The change

openssl `x509_vfy.c`, line 1308: the last argument of
`OCSP_basic_verify(bs, ctx->chain, ctx->store, OCSP_TRUSTOTHER)` changes from the identifier
`OCSP_TRUSTOTHER` to the literal `0`.

## Mapping

- Line 1308: the ground truth maps the `identifier` `OCSP_TRUSTOTHER` to the `number_literal` `0`
  in the same argument slot, as an update. omnidiff deletes the identifier and inserts the literal
  (op Delete, reason `APTED("qualified_name")`, the per-function APTED call in
  `solve_syntax_aware_matching.rs`). The two leaves are of different kinds, and
  `UnitCostModel::ren` (`src/diff/apted/common.rs`) prices a cross-kind pair above
  delete + insert unless `nodes::update_allowed_from_masks` allows the swap, so APTED never pairs
  them. Every other node of the call is mapped. The argument is the same
  slot with a new value, so an update is the reasonable reading. `matcher`.

## Painting

- Both presets, `Delete -> Update` (15 bytes, before) and `Insert -> Update` (1 byte, after) on line
  1308: the result of the delete + insert above. Absent with the human mapping. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the cross-kind update allowance (`nodes::update_allowed_from_masks`)
  for `identifier` -> `number_literal`, or a post-pass that pairs a lone deleted and a lone inserted
  leaf at the same position of a mapped parent (call site `APTED("qualified_name")`,
  `solve_syntax_aware_matching.rs`)
- Ground truth questionable: no
