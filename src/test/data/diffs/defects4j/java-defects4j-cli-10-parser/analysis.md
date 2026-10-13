# Accuracy analysis: java-defects4j-cli-10-parser

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 3516 nodes, 3516 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 26625: renderer 0, matcher 0 |
| Painting, Full | 2 bytes wrong of 26625: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-10: `Parser.setOptions` (before line 46, after line 47) copies the required options
into a new list, `new ArrayList(options.getRequiredOptions())`, instead of keeping the shared
list, and `java.util.ArrayList` is imported.

## Painting

No mapping mismatches; Minimal agrees with its painting. The Full runs also appear with the human
mapping, so they are the renderer's:

- Full, 2 bytes, Move -> None: the statement's closing `;` (before line 46, after line 47). It
  keeps its text and its place but is pushed right by the `new ArrayList(` wrapper; `FULL`'s
  `paint_displaced_moves` paints it `Move`, the Full painting does not. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` displaced-move painting (`paint_displaced_moves`) for a
  trailing `;` pushed by a wrapper
- Ground truth questionable: no
