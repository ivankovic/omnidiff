# Accuracy analysis: java-defects4j-cli-32-helpformatter

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 8012 nodes, 8012 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 62721: renderer 2, matcher 0 |
| Painting, Full | 0 bytes wrong of 62721: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-32 in `HelpFormatter.findWrapPos`: the `while` loop that advanced `pos` to the next
whitespace after `startPos + width` (before lines 937-941) is removed; the method now chops at
`startPos + width`.

## Painting

No mapping mismatches; Full agrees with its painting. The Minimal runs also appear with the human
mapping, so they are the renderer's:

- Minimal, 2 bytes, None -> Delete: `{` and `}` of the deleted `while` block (before lines 939,
  941). The Minimal painting paints them; `MINIMAL` leaves structural punctuation unpainted even
  in a wholly deleted block. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` `MINIMAL` structural-punctuation filter for braces of a wholly
  deleted block
- Ground truth questionable: no
