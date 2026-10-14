# Accuracy analysis: java-defects4j-cli-33-helpformatter

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 8162 nodes, 8162 graded, N:M floor 0 visible |
| Painting, Minimal | 58 bytes wrong of 64306: renderer 58, matcher 0 |
| Painting, Full | 32 bytes wrong of 64306: renderer 32, matcher 0 |
| Ground-truth invariant violations | 4 |

## The change

Defects4J Cli-33 in `HelpFormatter`: `printWrapped` calls a new `renderWrappedTextBlock` (line 730)
instead of `renderWrappedText`, and the new method (after lines 900-918) splits the text into lines
and wraps each one, catching the `IOException` that `BufferedReader.readLine` declares. The stray
`// cannot happen` comment that already existed lands inside the new `catch` block.

## Painting

No mapping mismatches. Every painting run also appears with the human mapping, and both groups are
flagged by `invariants.txt` (4 violations):

- Minimal, 34 bytes, None -> Update: `renderWrappedText` on line 730, both sides. The Minimal
  painting pairs the whole identifier `renderWrappedText` with `renderWrappedTextBlock` as an
  Update; omnidiff narrows it to the added `Block`. Invariant 16 flags exactly these two spans:
  "paints `renderWrappedText` against `renderWrappedTextBlock` wrongly for a Minimal reading - the
  differing words are `Block`". `ground-truth`.
- Minimal 24 / Full 32 bytes, Insert -> None: ` catch (IOException e) {` on after line 913 (with
  the line's indentation under Full). Both paintings' big Insert of the new method stops at the
  `}` in column 8 of that line and skips the new `catch` clause, although every other line of
  the method is painted. Invariant 11 flags `IOException` and `e` there in both paintings
  ("unpainted ... and the tree mapping says this is gone"). `ground-truth`.

## Verdict

- Primary cause: `ground-truth`
- Other causes: none
- Where a fix would go: ground truth (Marko's call)
- Ground truth questionable: yes: the Minimal painting updates the whole identifier on line 730
  (invariant 16), and both paintings leave the new `catch (IOException e) {` on after line 913
  unpainted (invariant 11)
