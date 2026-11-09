# Accuracy analysis: java-defects4j-cli-39-typehandler

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1784 nodes, 1784 graded, N:M floor 0 visible |
| Painting, Minimal | 6 bytes wrong of 15749: renderer 6, matcher 0 |
| Painting, Full | 0 bytes wrong of 15749: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-39 in `TypeHandler.createValue`: an `EXISTING_FILE_VALUE` option now returns
`openFile(str)` (line 92) instead of `createFile(str)`, and the new `openFile` (after lines
234-244) opens a `FileInputStream`, turning `FileNotFoundException` into a `ParseException`.

## Painting

No mapping mismatches; Full agrees with its painting. The Minimal runs also appear with the human
mapping, so they are the renderer's:

- Minimal, 6 bytes, None -> Insert: the `{` and `}` of the new method's blocks (after lines 235,
  237, 239, 241, 243, 244). The Minimal painting paints braces of inserted blocks; `MINIMAL`
  leaves structural punctuation unpainted even when the whole block is new. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` `MINIMAL` structural-punctuation filter for braces of an
  inserted block
- Ground truth questionable: no
