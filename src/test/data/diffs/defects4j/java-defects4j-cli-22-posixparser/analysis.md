# Accuracy analysis: java-defects4j-cli-22-posixparser

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1897 nodes, 1897 graded, N:M floor 0 visible |
| Painting, Minimal | 6 bytes wrong of 18975: renderer 6, matcher 0 |
| Painting, Full | 14 bytes wrong of 18975: renderer 14, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-22 in `PosixParser`: a `currentOption` field is tracked, `processNonOptionToken` gains
a `boolean stopAtNonOption` parameter and only stops when the current option takes no argument,
and the `else if (stopAtNonOption) { processNonOptionToken(token); }` branch (before lines 150-153)
is folded into the final `else`. All callers pass the new argument.

## Painting

No mapping mismatches. Every painting run also appears with the human mapping, so all are the
renderer's.

- Full, 14 bytes, Move -> None: the closing `);` (or `)`) after which an argument or parameter was
  added: `processNonOptionToken(token` + `, stopAtNonOption);` (before lines 116 and 148, after
  117 and 150), the method's parameter list (before 184, after 182) and
  `processNonOptionToken(token.substring(i)` + `, true);` (before 262, after 266). The kept
  closing tokens are pushed right by the insertion beside them; `FULL`'s `paint_displaced_moves`
  paints them `Move`, the Full painting leaves them unpainted. `renderer`.
- Minimal, 6 bytes, None -> Delete/Insert: the braces of the removed `else if` block (before
  lines 151, 153) and of the new `if` blocks (after lines 185-188, 213-215). The Minimal painting
  paints them; `MINIMAL` leaves structural punctuation unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` displaced-move painting (`paint_displaced_moves`) for a
  closing bracket after an appended argument; `MINIMAL` structural-punctuation filter for braces
  of wholly inserted or deleted blocks
- Ground truth questionable: no
