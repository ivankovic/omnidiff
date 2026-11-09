# Accuracy analysis: go-nwg-piotr-gopsuinfo-shuffle-around-if-blocks

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 91 (57 visible) of 5796 nodes, 5796 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

In `gopsuinfo.go`, a debug loop that printed every value is deleted (before lines 90-93) and a
comment is shortened (line 79). In the temperature fallback chain, the second and third levels of
the nested `if ... else { if ... }` swap places. The `coretemp_packageid0` check with
`fmt.Sprint(v)` (before lines 97-98) and the `coretemp_average` check with
`fmt.Sprint(int(math.Round(v)))` (before lines 100-101) trade nesting levels (after lines 93-94
and 96-97).

## Mapping

All 91 mismatches (57 visible) are on before lines 97-107, all from `APTED("qualified_name")`.
They are two groups of the same cause:

- The ground truth keeps each condition with its body. The `packageid0` if's `if`, its condition
  leaves, `{`, `output += fmt.Sprint(v)` and the closing `}` map Identical to their copies one
  level deeper (after lines 96-97). The `average` if maps to after lines 93-94. The `} else {`
  glue follows each one.
- Omnidiff maps the nesting levels in place. Before line 97's `if` goes to after line 93's `if`,
  the string content `coretemp_packageid0` is updated to `coretemp_average`, and the two
  `output += ...` bodies are updated into each other. The same happens one level down, and the
  `}` / `else` at lines 99, 101-103 and 106-107 follow by position.

APTED has no move operation, and pairing by nesting position costs two string relabels plus a
few leaf edits, which is cheaper than moving two subtrees. The ground truth's reading, that two
unchanged (condition, body) units swapped, is what the commit does. Omnidiff's reading shows a
reader two edited checks. `matcher`. It is close to `ambiguous`, because the positional reading
is a valid edit script.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: move detection after APTED at the `qualified_name` call site
  (`solve_syntax_aware_matching.rs`), for nested if/else-if levels whose condition+body units are
  identical but swapped.
- Ground truth questionable: no
