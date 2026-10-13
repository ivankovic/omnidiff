# Accuracy analysis: java-defects4j-closure-44-codeconsumer

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (1 visible) of 2634 nodes, 2634 graded, N:M floor 0 visible |
| Painting, Minimal | 3 bytes wrong of 13966: renderer 0, matcher 3 |
| Painting, Full | 21 bytes wrong of 13966: renderer 12, matcher 11 |
| Ground-truth invariant violations | none |

## The change

In the `if ((isWordChar(c) || c == '\\') && isWordChar(getLastChar()))` block (lines 189-199), the
five comment lines about a forward slash after a DIV now belong to a new `} else if (c == '/' &&
getLastChar() == '/') {` branch (line 194 after), which also gains an `append(" ");` (line 200
after).

## Mapping

- Line 190 `{` ... line 199 `}`, the then-block of the `if`: the ground truth maps it to the after
  then-block (lines 190-194), and its `}` (line 199) to the `}` of `} else if` on line 194. omnidiff
  maps neither: `APTED("qualified_name")` deletes the before block and its `}` (1 visible, the `}`;
  1 invisible, the block). The block's trailing comments moved out into the new else-if block, which
  leaves the then-block with little in common beyond `append(" ");`, and the APTED fallback did not
  keep the container. `matcher`.

## Painting

Matcher-caused (absent with the human mapping):

- Minimal, `Delete -> None` on line 199 `}` (1 byte) and `Insert -> None` on line 194 `} ` (2
  bytes): the unmapped then-block's brace is painted deleted and re-inserted. `matcher`.
- Full, the same two runs with their indentation (`    }` and `    } `, 11 bytes). `matcher`.

Renderer-caused (also present with the human mapping), Full only, scored against the single
`Minimal` painting:

- `Insert -> None` on the indentation of the inserted lines 200 and 201 (6 + 4 bytes): Full paints
  indentation, the Minimal painting does not. `grading-artifact`.
- With the human mapping, Full paints the then-block `}` Move on both sides (2 bytes), because it
  moved from line 199 to line 194. This is Full's convention; the Minimal painting leaves it alone.
  `grading-artifact`.

## Verdict

- Primary cause: `matcher`
- Other causes: `grading-artifact`
- Where a fix would go: APTED fallback (call site qualified_name) keeping a block container whose
  tail children moved to a sibling block
- Ground truth questionable: no
