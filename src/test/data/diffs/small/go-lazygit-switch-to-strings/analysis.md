# Accuracy analysis: go-lazygit-switch-to-strings

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 17 (7 visible) of 482 nodes, 482 graded, N:M floor 0 visible |
| Painting, Minimal | 37 bytes wrong of 1443: renderer 28, matcher 48 |
| Painting, Full | 59 bytes wrong of 1443: renderer 41, matcher 58 |
| Ground-truth invariant violations | none |

## The change

In lazygit's submodule presentation, the indentation is no longer built by appending. Before:
`indentation := ""`, then `indentation += "  "` in a for loop over parent modules. After:
`count := 0`, `count++` in the loop, then `indentation := strings.Repeat("  ", count)` after the
loop. A `"strings"` import is added.

## Mapping

All 17 mismatches come from `APTED("qualified_name")` over the function.

- Before line 18 and after lines 20 and 24 (5 visible, 7 invisible). The ground truth moves
  `indentation :=` from line 18 to after line 24, and inserts `count := 0`. Omnidiff updates
  `indentation := ""` in place into `count := 0` (identifier `indentation` -> `count`, `:=` kept).
  It then maps the `indentation` of the deleted `indentation += "  "` (before line 20) to the
  `indentation` of the new declaration on after line 24. APTED pairs by position and has no
  move. `matcher`.
- Before lines 19-21 (2 visible, 3 invisible): the for loop's `{` and `}`, its `block` and
  `statement_list`, and the `for_statement` itself are deleted. The loop header's leaves are not
  in the mismatch list, so they stay matched. The block's only statement changed completely
  (`indentation += "  "` -> `count++`), and APTED dropped the glue around it. `matcher`.

## Painting

- Minimal, matcher, 33 bytes. `indentation` on before line 18 is painted Update (the painting:
  Move). `indentation` on before line 20 is painted Move (the painting: Delete). `count := ` on
  after line 20 is painted Update (the painting: Insert).
- Minimal, renderer, 4 bytes: ` := ` of the moved declaration on after line 24 is left
  unpainted. With the human mapping, the whole `indentation :=` is unpainted on both sides
  (`None -> Move`, 28 bytes). The declaration moves past the for loop but stays at the same
  column, and `identical_or_move` does not count a row-only move past a sibling as a Move. The
  painting does. `renderer`.
- Full: about 31 bytes are the matcher's (the loop's braces painted Delete/Insert, `count := `
  left as an update). About 28 bytes are the renderer's, and are also present with the human
  mapping: the moved declaration and indentation tabs not painted Move, and one deleted space.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: the APTED call at `qualified_name` (`solve_syntax_aware_matching.rs`). It
  pairs declarations of different variables by position, and deletes a loop block whose body was
  replaced. In the renderer, a same-column declaration that moved past a sibling statement should
  be painted Move.
- Ground truth questionable: no
