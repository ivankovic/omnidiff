# Accuracy analysis: java-defects4j-cli-21-writeablecommandlineimpl

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 4146 nodes, 4146 graded, N:M floor 0 visible |
| Painting, Minimal | 8 bytes wrong of 20615: renderer 8, matcher 0 |
| Painting, Full | 0 bytes wrong of 20615: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-21 in `WriteableCommandLineImpl`: a `currentOption` field with getter and setter and
a `checkForOption` re-entrancy guard are added, and `looksLikeOption` is wrapped in `if
(checkForOption != null) { ... }` / `try { ... } finally { ... }` and only returns `true` when the
current option can process the trigger (after lines 243-271).

## Painting

No mapping mismatches; Full agrees with its painting. The Minimal runs also appear with the human
mapping, so they are the renderer's:

- Minimal, 8 bytes, None -> Insert: the `{` and `}` of the new blocks in `looksLikeOption` (after
  lines 244, 247, 251, 260, 262, 267, 269, 271). The Minimal painting paints the braces of
  inserted blocks; `MINIMAL` leaves structural punctuation unpainted even when the whole block is
  new. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` structural-punctuation filter under `MINIMAL`
  (`structural_punctuation`): braces of an inserted block
- Ground truth questionable: no
