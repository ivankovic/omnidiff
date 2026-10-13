# Accuracy analysis: ruby-jmespath-jmespath-go-from-conditional-to-unless

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 27 (10 visible) of 877 nodes, 877 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In a compliance spec, the wrapper `if test_case['error'] ... end` (before lines 11 and 39) is
replaced by a guard `next unless test_case['error']` (after line 11), and its body is de-indented
one level. Also `:disable_visit_errors => true` becomes `disable_visit_errors: true` and `%w[...]`
becomes `%w(...)`. description.md: "The deleted end is mateched to the if".

## Mapping

- **Nested `describe`/`each do` containers unmapped** (16 invisible, `UnresolvedNode`; 1
  invisible, `APTED("bottom_up_propagation")`). The chain of `call`/`do_block`/`body_statement`
  from line 4 to line 13 has no pair. None of these are named items, so no APTED call per item
  reaches them, and the residual whole-file pass (`fast_fallback`,
  `resolve_residual_forest_via_myers_lcs`) works on a sequence of leftover nodes.
- **Keywords paired by sequence, one level off** (9 visible, `APTED("fast_fallback")`). Before has
  one more `if` and one more `end` than after. The ground truth deletes the outer `if` (line 11)
  and its `end` (line 39) and keeps every other keyword at its nesting level. The LCS fallback
  pairs from the front: the outer `if` on line 11 takes the after `if` on line 13, and the inner
  `if` on line 13 is deleted. The `end`s on before lines 39-44 take after lines 37-42, each one
  nesting level out from the ground truth, and the outermost `end` (line 45) is deleted. A reader
  sees the wrong `if` and the wrong `end` as deleted; the shifted `end`s are the likely reason
  the container chain above cannot be resolved bottom-up (each `do_block`'s `end` now points into
  a different after block; not verified further).
- **`:disable_visit_errors` -> `disable_visit_errors:`** on line 4 (1 visible,
  `APTED("fast_fallback")`): the `simple_symbol` is deleted, the ground truth maps it to the
  `hash_key_symbol`. Kind change. `matcher`.

All three groups are `matcher`; the ground truth's nesting-preserving reading is the reasonable
one.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the residual LCS fallback (`resolve_residual_forest_via_myers_lcs`,
  src/diff/apted/common/resolve.rs) pairing keyword leaves from the front without nesting; or an
  earlier pass that matches the unnamed nested `describe ... do` / `each do` calls so the
  fallback is never reached here.
- Ground truth questionable: no
