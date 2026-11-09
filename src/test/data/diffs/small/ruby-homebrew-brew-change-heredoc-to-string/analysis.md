# Accuracy analysis: ruby-homebrew-brew-change-heredoc-to-string

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (0 visible) of 4207 nodes, 4207 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `cask/upgrade.rb` (lines 259-262) the heredoc `oh1 <<~EOS ... EOS` becomes two calls:
`oh1 "Upgrading #{...}"` and `puts "  #{old_cask.version} -> #{new_cask.version}"`.

## Mapping

Both mismatches are invisible wrapper nodes on before line 259: the `call` `oh1 <<~EOS` and its
`argument_list` `<<~EOS`. The ground truth maps them to the after `oh1 "..."` call and its argument
list. Omnidiff leaves them unmapped (Delete, reason `APTED("qualified_name")`, the APTED call for
the enclosing `singleton_method`). The `oh1` identifier and every interpolation are mapped as the
ground truth expects, so nothing a reader sees differs.

Why: tree-sitter-ruby puts the `heredoc_body` beside the `call` (both are children of `begin`),
not inside it. The ground truth maps the heredoc's interpolations into the string inside the after
`argument_list`. With those pairs in place, mapping the before `argument_list` to the after one
breaks the ancestor rule of an ordered tree edit mapping: the after list contains the
interpolations, but the before list does not. APTED therefore cannot return both, and it keeps
the leaf pairs. The ground truth is a reasonable reading of the change. The disagreement is only
on invisible wrappers.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: `matcher`
- Where a fix would go: if wanted, a post-pass that maps an unmapped `call`/`argument_list` whose
  identifier child is mapped (bottom-up propagation), outside APTED's ancestor rule. No visible gain.
- Ground truth questionable: no
