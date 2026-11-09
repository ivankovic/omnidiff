# Accuracy analysis: rust-vercel-nextjs-refactoring-would-require-mulitmap-mapping

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 10 (8 visible) of 18162 nodes, 17899 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`turbo-tasks-macros/src/func.rs`: the nested `if *is_method { ... if *is_self_used { ... } }`
that built a `NativeFunction` is rewritten as `let task_fn = if *is_method && *is_self_used`,
then `let arg_meta = ...` built from the old `arg_filter` code, then one `quote!`.
`description.md`: "Requires N:M multi-map." The ground truth as filed is 1:1 here (floor 0), and
it grades 17899 of 18162 nodes.

## Mapping

- **The merged condition, 2 visible + 2 invisible.** Ground truth maps the outer
  `if *is_method { ... }` (before line 1107) to the new `if *is_method && *is_self_used`
  (after line 1107), and also the inner condition `*is_self_used` (before line 1123) into the new
  `&&`. omnidiff deletes both the outer `if_expression` and `*is_self_used`
  (`APTED("qualified_name")`): the new `if` shares almost nothing with the old body, and the inner
  condition now sits in a different `if`. `matcher`.
- **`#` of `#filter_owned`, 2 visible.** Before line 1115 `#filter_owned` -> after line 1121
  `#filter_owned` in the ground truth; omnidiff pairs the before `#` with the new `#` of
  `&#task_fn` (after line 1120) instead (`APTED("qualified_name")`, op Identical), so the new one
  is not shown as an insert. Wrong twin among identical `#` tokens in a `token_tree`. `matcher`.
- **Nested parentheses `((` ... `))` -> `(` ... `)`, 4 visible.** Before lines 1114-1117
  `Option::Some((#filter_owned, #filter_and_resolve,))` become
  `with_filter_trait_call_from(&#task_fn, #filter_owned, #filter_and_resolve,)`. The ground truth
  keeps the inner pair (the one around the arguments) and deletes the outer; omnidiff keeps the
  outer and deletes the inner (`APTED("qualified_name")`). Same cost either way; the ground
  truth's choice keeps the pair whose contents survived. `ambiguous`.

## Verdict

- Primary cause: `matcher`
- Other causes: `ambiguous`
- Where a fix would go: the `qualified_name` APTED call site (`solve_syntax_aware_matching`):
  keeping an `if_expression` whose condition was extended, and twin choice among `#` tokens
- Ground truth questionable: no
