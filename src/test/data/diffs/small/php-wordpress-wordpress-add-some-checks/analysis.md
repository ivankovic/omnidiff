# Accuracy analysis: php-wordpress-wordpress-add-some-checks

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 10 (6 visible) of 69723 nodes, 69723 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`wp_maybe_inline_styles()` in `script-loader.php` tightens its checks: `if ( ! $size )` (line
3013) becomes `if ( 0 === $size && ! file_exists( $path ) )` with a `_doing_it_wrong(...)` call
before `continue`, and a new `if ( ! is_readable( $style['path'] ) )` guard with the same kind of
call is added before `file_get_contents`.

## Mapping

All 10 mismatches are on line 3013, reason `APTED("qualified_name")` (the APTED run inside the
matched function).

- The ground truth deletes `! $size`'s `!` and its `unary_op_expression`, keeps `$size` Identical
  as the right operand of the new `0 === $size`, and treats `! file_exists( $path )` as wholly new.
- omnidiff keeps the unary expression and maps it to the new `! file_exists( $path )`
  (MatchButNotIdentical): the `!` becomes Identical to the new `!`, and `$size` is mapped to
  `$path` as an Update of the name (`size` -> `path`). This keeps the tree shape (unary over a
  variable) at the price of a rename, while an identical `$size` sits one operand to the left.
  The ground truth's reading (the variable is the same, the negation went away) is the faithful
  one. `matcher`.

Visible: `!`, `$` and `size`/`path` on both sides (6); invisible: the two `unary_op_expression`
and two `variable_name` containers (4).

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED within matched functions (`solve_syntax_aware_matching`,
  `qualified_name`): an identical variable should win over a renamed one that only preserves the
  wrapper's shape.
- Ground truth questionable: no
