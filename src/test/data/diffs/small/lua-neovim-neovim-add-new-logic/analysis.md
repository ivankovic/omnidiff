# Accuracy analysis: lua-neovim-neovim-add-new-logic

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 10 (5 visible) of 32540 nodes, 32540 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In the `float_pos` branch of `test/functional/ui/screen.lua`, the commit drops `.. v[1]` from line
1798 (`str = str .. '  [' .. k .. '] = {' .. v[1]`). It adds an `if v.external then ... else ...
end` whose `else` branch holds `str = str .. v[1]` and the existing `for` loop, re-indented (after
lines 1798-1806).

## Mapping

- **Truncated concatenation deleted instead of updated (5 visible, 5 invisible), `matcher`.** The
  ground truth maps before line 1798's assignment to after line 1798, with `.. v[1]` removed.
  omnidiff deletes the before `assignment_statement`, its `expression_list` and the four
  `binary_expression` levels (`APTED("qualified_name")`, op Delete). It also deletes the leaves
  `str` (the right-hand one), the three `..` and `k`. The string literals `'  ['` and `'] = {'` and
  the rest of the function agree with the ground truth. Lua's `..` is right-associative, so removing
  the last operand re-nests every `binary_expression` level: before `str .. ('  [' .. (k .. ('] = {'
  .. v[1])))`, after `str .. ('  [' .. (k .. '] = {'))`. The APTED run over the enclosing function
  then pairs some leaves and gives up on the chain. I did not trace the APTED costs to confirm why
  the statement itself goes unmatched, which is undetermined. A reader sees the line deleted and
  re-inserted instead of shortened.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: `APTED("qualified_name")` (src/diff/solve_syntax_aware_matching.rs) or a
  statement-level pre-pairing of same-target assignments (`str = ...`) before APTED. Right-nested
  `..` chains that lose a tail operand need the statement anchored first.
- Ground truth questionable: no
