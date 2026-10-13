# Accuracy analysis: java-defects4j-cli-28-parser

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 3360 nodes, 3356 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 26071: renderer 13, matcher 13 |
| Painting, Full | 40 bytes wrong of 26071: renderer 53, matcher 13 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-28 in `Parser.processProperties`: a property whose value is not yes/true/1 now
skips to the next property (`continue;`) instead of ending the loop (`break;`), line 290. Marko's
`description.md`: "Two equally valid preferences." There are three paintings: `Minimal (prefer
Insert/Delete)`, `Minimal (prefer Update)` and `Full (prefer Insert/Delete)`.

## Mapping

No mismatches, but 4 of 3360 nodes are ungraded: the human tree lists only the `;` of the
statement, leaving `break`/`continue` and their statements unlisted, which is how it leaves both
preferences open. The 0 is vacuous for exactly those nodes, by design.

## Painting

Minimal agrees with one of its paintings. Full:

- Full, 40 bytes, None -> Move: the 20-space indentation of line 290, on both sides. The Full
  painting contains `Match 290:0-290:20 -> 290:0-290:20`: the indentation matched to itself at
  the same row and column. The grader scores a byte-identical match as `Move`, so the in-place,
  unchanged indentation counts as moved; omnidiff leaves it unpainted. The same run appears with
  the human mapping. Nothing there moved. `ground-truth`.

## Verdict

- Primary cause: `ground-truth`
- Other causes: none
- Where a fix would go: ground truth (Marko's call), or the grader's reading of a same-position
  identical `Match` as `Move`
- Ground truth questionable: yes: the Full painting matches line 290's indentation to itself in
  place, which scores as 40 bytes of `Move`
