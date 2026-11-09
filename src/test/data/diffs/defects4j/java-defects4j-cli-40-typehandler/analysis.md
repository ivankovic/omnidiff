# Accuracy analysis: java-defects4j-cli-40-typehandler

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 1958 nodes, 1958 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 16323: renderer 2, matcher 2 |
| Painting, Full | 24 bytes wrong of 16323: renderer 26, matcher 26 |
| Ground-truth invariant violations | none |

## The change

Cli-40: in `TypeHandler.createValue`, the fall-through branch for an unknown class changes from
`return null;` to `throw new ParseException("Unable to handle the class: " + clazz);` (line 103).
Nothing else changes.

## Mapping

- Line 103, the statement terminator `;`. The ground truth deletes the `return_statement` and
  inserts the `throw_statement` with all their children, `;` included. omnidiff maps the before `;`
  (103:24) to the after `;` (103:77), as Identical through `APTED("qualified_name")`. Both of the
  ground truth's own paintings leave this `;` unpainted on both sides (Minimal and Full spans end at
  columns 23 and 76), which is what omnidiff's mapping renders. So the tree mapping and the painting
  of the same file disagree about the `;`; omnidiff agrees with the painting. Matching a terminator
  across a replaced statement is defensible either way. `ground-truth` (2 visible).

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Painting

- Minimal: no disagreement. Rendering the human mapping would paint the two `;` Delete/Insert
  (2 bytes) where the painting leaves them alone; omnidiff's mapping avoids that.
- Full, 24 bytes, None -> Move, line 103 columns 0-12 on both sides: the Full painting has an
  explicit `Match` of the unchanged 12-byte indentation to itself. Byte-identical sides grade as
  Move, while omnidiff leaves unchanged indentation unpainted. The indentation did not move or
  change; the disagreement comes from how a same-position identical Match is graded.
  `grading-artifact`. With the human mapping the renderer paints this indentation Delete/Insert
  (26 renderer bytes, with the `;`), which is further from the painting than omnidiff's own.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: `ground-truth`
- Where a fix would go: none in the engine; the `;` is the ground truth's call (Marko's), and the
  indentation is the grader's treatment of an in-place identical Match.
- Ground truth questionable: yes: the tree mapping deletes/inserts the line-103 `;` with its
  statements, while both paintings leave that `;` unpainted, i.e. kept.
