# Accuracy analysis: vimscript-junegunn-fzf-condition-canges

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (0 visible) of 18231 nodes, 18231 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

One condition on line 361 of fzf.vim: `... && has('termguicolors') && &termguicolors` becomes
`... && (has('gui_running') || has('termguicolors') && &termguicolors)`. This re-associates the
left-nested `&&` chain: `has('termguicolors') && &termguicolors` becomes its own subexpression
under a new `||`.

## Mapping

- 1 invisible, 0 visible. The ground truth puts the two before `binary_operation` nodes that change
  shape (`!s:is_win && !has('win32unix') && has('termguicolors')` and the whole `... &&
  &termguicolors`) and three after nodes (the outer `&&`, the new `||`, and `has('termguicolors') &&
  &termguicolors`) into one MatchButNotIdentical group, AnyOneToOne. So any two pairs count.
  omnidiff realizes one pair and leaves the before node `!s:is_win && !has('win32unix') &&
  has('termguicolors')` unmatched. The evidence does not print omnidiff's reason for group members.
  Every leaf in the line agrees with the ground truth, and a `binary_operation` has no text of its
  own, so nothing visible differs. `grading-artifact`, with an underlying `matcher` choice.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: `matcher`
- Where a fix would go: none needed for what a reader sees. For the invisible count: pairing of
  `binary_operation` containers when an `&&` chain is re-associated.
- Ground truth questionable: no
