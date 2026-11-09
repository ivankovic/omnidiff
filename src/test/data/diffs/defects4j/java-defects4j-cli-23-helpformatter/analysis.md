# Accuracy analysis: java-defects4j-cli-23-helpformatter

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 7 (6 visible) of 7698 nodes, 7700 graded, N:M floor 2 visible |
| Painting, Minimal | 28 bytes wrong of 59488: renderer 3, matcher 27 |
| Painting, Full | 67 bytes wrong of 59488: renderer 11, matcher 60 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-23 in `HelpFormatter.renderWrappedText`: the `lastPos` infinite-loop guard that threw
`RuntimeException("Text too long for line ...")` (before lines 834-837) is replaced by `if (
(text.length() > width) && (pos == nextLineTabStop - 1) ) { sb.append(text); return sb; }`
(after lines 834-838), and `int lastPos = pos;` is removed. Marko's `description.md`: "Requires
N:M mapping. Low confidence solution as is." The best-matching paintings are the `(Outer
parenthesis)` ones.

## Mapping

- **`throw new RuntimeException(...)` -> `sb.append(text);` (4 visible).** Before line 836, after
  line 835. The ground truth deletes the `throw` and inserts `sb.append(text);`. omnidiff pairs
  the exception type `RuntimeException` (a `type_identifier`) with the variable `sb` (an
  `identifier`) and the two `;`, through `APTED("large_flat_subtree")`, the same call site that
  leaves the enclosing `if` unmatched below. The statements share nothing a reader would call
  the same. `matcher`.
- **`- 1` of `nextLineTabStop - 1` (2 visible).** The ground truth maps `lastPos` (before line
  833) to `nextLineTabStop - 1` as a 1:3 all-to-all group; one-to-one, omnidiff leaves `-` and
  `1` inserted (`APTED("large_flat_subtree")`). This is the fixture's N:M floor (2 visible).
  `nm-floor`.
- **The `if (pos == -1)` statement (1 invisible).** Before line 828: omnidiff leaves the container
  unmatched (op Delete, `APTED("large_flat_subtree")`) while its contents are matched. No
  visible effect. `grading-artifact`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Painting

- Matcher-caused, Minimal 21 / Full 53 bytes: ` RuntimeException` and `;` of the `throw` (before
  line 836, with its indentation under Full) not painted Delete, and `sb` / `;` of after line 835
  painted Update/Move where the painting inserts them.
- N:M-caused, Minimal 5 / Full 4 bytes, None -> Update: ` - 1` (after line 834), which the
  painting paints as part of the `lastPos` -> `nextLineTabStop - 1` update. `nm-floor`.
- Renderer-caused (also with the human mapping), Minimal 2 / Full 10 bytes: the parentheses of the
  old and new conditions on line 834 (the Full painting moves the old `(`, `)` to the new outer
  pair; the Minimal painting paints the new inner ` (` Insert, omnidiff does not), and the
  `{` that moved from its own line (before 835) onto the `if` line (after 834), which omnidiff
  paints `Move` and the Full painting does not. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `nm-floor`, `renderer`, `grading-artifact`
- Where a fix would go: the `large_flat_subtree` APTED call site (`solve_large_flat_subtrees`),
  which pairs a `type_identifier` with an `identifier` across a deleted `throw` and an inserted
  call; N:M support (`solve_multi_maps`)
- Ground truth questionable: no (Marko marks the solution low-confidence himself)
