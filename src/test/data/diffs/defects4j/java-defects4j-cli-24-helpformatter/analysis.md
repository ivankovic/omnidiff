# Accuracy analysis: java-defects4j-cli-24-helpformatter

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 7746 nodes, 7746 graded, N:M floor 0 visible |
| Painting, Minimal | 38 bytes wrong of 60145: renderer 0, matcher 38 |
| Painting, Full | 65 bytes wrong of 60145: renderer 15, matcher 50 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-24 in `HelpFormatter.renderWrappedText`: when `nextLineTabStop >= width`, the method
no longer throws `IllegalStateException("Total width is less than ...")` (before lines 825-826) but
clamps the tab stop, `nextLineTabStop = width - 1;` (after line 825).

## Mapping

- **`IllegalStateException` -> `nextLineTabStop` (2 visible).** Before line 825 col 23, after line
  825 col 13. The ground truth deletes the `throw` and inserts the assignment. omnidiff pairs
  the exception type (a `type_identifier`) with the assigned variable (an `identifier`),
  MatchButNotIdentical, reason `APTED("large_flat_subtree")`. Nothing else of the two statements
  is paired, so this is a lone cross-kind pairing. `matcher`.

## Painting

Matcher-caused (absent with the human mapping), Minimal 38 / Full 50 bytes: ` IllegalStateException`
(before line 825) not painted Delete, and `nextLineTabStop ` (after line 825) painted Update instead
of Insert; under Full also the indentation of before line 825.

Renderer-caused (also with the human mapping), Full 15 bytes:

- None -> Insert, 12 bytes: the indentation of the new assignment (after line 825), which the Full
  painting includes in the insert. `renderer`.
- Move -> None, 2 bytes: the `;` ending the old and the new statement (before line 826, after
  line 825), paired and painted `Move`; the Full painting leaves it. `renderer`.
- Delete -> None, 1 byte: the trailing space after `+` at the end of before line 825, painted with
  the deleted string concatenation; the Full painting stops before it. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: `APTED("large_flat_subtree")` (`solve_large_flat_subtrees`): reject a lone
  `type_identifier` <-> `identifier` match between a deleted and an inserted statement
- Ground truth questionable: no
