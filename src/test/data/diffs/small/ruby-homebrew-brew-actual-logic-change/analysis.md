# Accuracy analysis: ruby-homebrew-brew-actual-logic-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 13 (6 visible) of 9576 nodes, 9576 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `bump.rb` the statement `current_versions << " (deprecated)" if formula_or_cask.deprecated?`
(before line 487) is removed. Instead, each branch of the `current_versions = if ... else ... end`
assignment gets a new `"#{" (deprecated)" if deprecated[:arm|:intel|:general]}"` string part, and
the `else` branch's single string is split into a three-part chained string (after lines 483-497).

## Mapping

All 13 mismatches have one cause: the APTED call that `solve_qualified_name_groups` makes for the
enclosing method (reason `APTED("qualified_name")`).

- Omnidiff maps the removed before `if_modifier` on line 487 (`current_versions << " (deprecated)"
  if formula_or_cask.deprecated?`) and its `if` keyword to the new after `if_modifier`
  `" (deprecated)" if deprecated[:general]` on line 496, inside the after assignment. The ground
  truth has the first deleted and the second inserted. (2 visible `if`, 2 invisible
  `if_modifier`.)
- To make that pairing, ordered tree edit distance must keep ancestry. The before statement is a
  sibling of the assignment, but its after partner is inside the assignment. So APTED gives up
  the pair of the assignment and its structure. The before `assignment`, `if`, `then`, `else` on
  line 477-486 are unmapped (Delete). The visible effect is that the `chained_string` on line 478,
  the `else` keyword on line 483 and the `end` on line 486 are shown as deleted, not kept. The
  `"#{current_version.general}..."` string on line 485 is unmapped, but the ground truth maps it
  1:2 to the two after parts (multi-map group; one pair is available to a 1:1 mapping, so the
  floor is 0).
- Cause: `matcher`. APTED takes a cheap leaf-level match (the shared `" (deprecated)"` / `if`
  text) at the cost of the containing structure. The ground truth is reasonable: the removed
  statement and the new interpolations are different constructs in different places.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the APTED call in `solve_qualified_name_groups`
  (src/diff/solve_syntax_aware_matching.rs), or a later pass that restores the pair of an unmapped
  `assignment`/`if` whose children are mostly matched to one after container.
- Ground truth questionable: no
