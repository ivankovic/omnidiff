# Accuracy analysis: java-defects4j-closure-131-tokenstream

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 4874 nodes, 4874 graded, N:M floor 0 visible |
| Painting, Minimal | 5 bytes wrong of 17317: renderer 0, matcher 5 |
| Painting, Full | 7 bytes wrong of 17317: renderer 0, matcher 7 |
| Ground-truth invariant violations | none |

## The change

Closure-131: `TokenStream.isJSIdentifier` rejects identifier-ignorable characters: a new operand
line `Character.isIdentifierIgnorable(s.charAt(0)) ||` is inserted into the `if` condition
(after line 194), and a second `if (` gains the same test for `s.charAt(i)` (line 199 -> 200).

## Mapping

- The `||` operators of `length == 0 || !Character.isJavaIdentifierStart(...)`, 2 visible. The
  chain is left-nested: after the insertion the before chain's outer `||` is structurally the
  outer `||` at the end of the new line 194, while textually it stays at the end of line 193.
  The ground truth follows the text: the line-193 `||` is kept and the new line's `||` is
  inserted with it. omnidiff follows the tree (op Identical, `APTED("qualified_name")`): it maps
  the before `||` to the new line's `||` and inserts the line-193 one. A reader sees one inserted
  line with its trailing `||`; omnidiff instead shows an unchanged-looking new line end and a new
  `||` on an untouched line. `matcher`.

## Painting

All disagreement is matcher-owned (no runs with the human mapping).

- Minimal 5 bytes / Full 7: the line-193 `||` painted Insert (and, in Full, the before one Move)
  where the painting leaves it alone; the new line's ` ||` left unpainted where the painting has
  Insert. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED("qualified_name") in `src/diff/solve_syntax_aware_matching.rs`:
  operand insertion into a left-nested binary chain should keep identical operators at their
  textual position.
- Ground truth questionable: no.
