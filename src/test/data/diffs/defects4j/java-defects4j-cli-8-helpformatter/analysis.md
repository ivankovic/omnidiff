# Accuracy analysis: java-defects4j-cli-8-helpformatter

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 7526 nodes, 7526 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 58958: renderer 16, matcher 16 |
| Painting, Full | 0 bytes wrong of 58958: renderer 16, matcher 16 |
| Ground-truth invariant violations | none |

## The change

Cli-8: in `HelpFormatter.renderWrappedText`, the wrap position of continuation lines is computed
with tab stop `0` instead of `nextLineTabStop` (line 812:
`pos = findWrapPos(text, width, 0);`). One argument changes.

## Mapping

- Line 812, the third argument. The ground truth's tree mapping matches the `identifier`
  `nextLineTabStop` to the `decimal_integer_literal` `0` (MatchButNotIdentical). omnidiff deletes
  the identifier and inserts the literal (op Delete, reason `APTED("large_flat_subtree")`). The
  ground truth's own painting ("Only one solution") paints exactly that: `nextLineTabStop` Delete,
  `0` Insert. omnidiff's mapping renders the painting byte for byte; the human tree mapping
  renders both as Update (15 + 1 renderer bytes). The tree mapping and the painting of the same
  fixture disagree, and omnidiff sides with the painting. `ground-truth` (1 visible).

## Painting

No disagreement under either preset with omnidiff's mapping. The 16 renderer bytes are the human
mapping's Update against the painting's Delete/Insert, described above.

## Verdict

- Primary cause: `ground-truth`
- Other causes: none
- Where a fix would go: ground truth (Marko's call); nothing in the engine.
- Ground truth questionable: yes: line 812's identifier -> integer literal is a Match in the tree
  mapping but a Delete + Insert in the painting.
