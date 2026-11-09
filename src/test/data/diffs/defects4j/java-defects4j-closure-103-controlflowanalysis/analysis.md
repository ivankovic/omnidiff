# Accuracy analysis: java-defects4j-closure-103-controlflowanalysis

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 16 (10 visible) of 13198 nodes, 13198 graded, N:M floor 0 visible |
| Painting, Minimal | 40 bytes wrong of 66141: renderer 0, matcher 40 |
| Painting, Full | 53 bytes wrong of 66141: renderer 0, matcher 53 |
| Ground-truth invariant violations | none |

## The change

Closure-103: `ControlFlowAnalysis.mayThrowException` adds `case Token.INSTANCEOF:` (after line
894) to the fall-through list of `case` labels that return `true`, right after `case Token.DEC:`.

## Mapping

All 16 mismatches (10 visible, 6 invisible) are one decision, reason
`APTED("large_flat_subtree")`.

- tree-sitter-java groups the last label with the statement: before, group 8 is
  `case Token.DEC: return true;`; after, `case Token.DEC:` is a group of its own and the new group
  9 is `case Token.INSTANCEOF: return true;`. The ground truth keeps `case Token.DEC:` on line
  893 -> 893 and inserts `case Token.INSTANCEOF:` on line 894. omnidiff maps the before group
  (with `return true;`) to the new after group 9, updates `DEC` -> `INSTANCEOF`, and inserts the
  after `case Token.DEC:` as new. APTED keeps the larger subtree (the one with `return true;`)
  together across the regrouping instead of keeping the identical label. Line 893 reads
  "DEC renamed to INSTANCEOF, DEC inserted above" where the source shows one inserted line.
  `matcher`.

## Painting

All disagreement is matcher-owned (no runs with the human mapping).

- Minimal, 40 bytes: before `DEC` painted Update (3), after line 893 `case Token.DEC:` painted
  Insert (15), after line 894 `case Token.INSTANCEOF:` left unpainted where the painting has
  Insert (22). `matcher`.
- Full, 53 bytes: the same runs with the indentation and `:` included. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the `large_flat_subtree` APTED pass
  (`src/diff/solve_large_flat_subtrees.rs`): identical `switch_label`s should anchor before a
  statement group is matched across a regrouping.
- Ground truth questionable: no.
