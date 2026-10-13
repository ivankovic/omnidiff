# Accuracy analysis: java-defects4j-cli-29-util

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 494 nodes, 494 graded, N:M floor 0 visible |
| Painting, Minimal | 38 bytes wrong of 4539: renderer 38, matcher 0 |
| Painting, Full | 14 bytes wrong of 4539: renderer 14, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-29 in `Util.stripLeadingAndTrailingQuotes`: the two independent strips (leading `"`,
then trailing `"`, before lines 65-72) are replaced by one `if (length > 1 &&
str.startsWith("\"") && str.endsWith("\"") && str.substring(1, length - 1).indexOf('"') == -1)`
that strips both quotes only when there is no inner quote (after lines 66-69).

## Painting

No mapping mismatches. Every painting run also appears with the human mapping, so all are the
renderer's.

- Minimal, 36 bytes, Move -> None: `str.endsWith("\"")` (before line 70, after line 66). It went
  from being the whole condition to the third operand of the new `&&` chain. omnidiff paints it
  `Move`; the Minimal painting leaves it unpainted (it sits in the same `if` with the same text).
  `renderer`.
- Minimal, 2 bytes, None -> Delete: `{` and `}` of the deleted first `if` block (before lines 66,
  68), which the Minimal painting paints and `MINIMAL` leaves as structural punctuation.
  `renderer`.
- Full, 14 bytes: the indentation of the deleted `if` line (before line 65, 8 bytes, None ->
  Delete), four single spaces between the inserted operands (after line 66, None -> Insert),
  and the condition's closing `)` (before 70, after 66) painted `Move` as displaced. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` move painting under `MINIMAL` for an operand that keeps its
  `if` but is nested deeper in a grown condition; gap whitespace and displaced `)` under `FULL`
- Ground truth questionable: no
