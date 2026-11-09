# Accuracy analysis: java-defects4j-cli-31-helpformatter

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 21 (15 visible) of 8070 nodes, 8077 graded, N:M floor 5 visible |
| Painting, Minimal | 98 bytes wrong of 62704: renderer 0, matcher 98 |
| Painting, Full | 97 bytes wrong of 62704: renderer 3, matcher 98 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-31 in `HelpFormatter.appendOption`: the argument-name test `option.hasArgName()`
(line 653) becomes `(option.getArgName() == null || option.getArgName().length() != 0)`, and the
appended name `option.getArgName()` (line 656) becomes `option.getArgName() != null ?
option.getArgName() : getArgName()`, falling back to the formatter's default. Marko's
`description.md`: "Requires N:M mapping. I accept that this could be a controversial solution."

## Mapping

- **The copied `option.getArgName()` on line 656 (5 visible, 2 invisible).** The ground truth maps
  the before call to both after copies as a 1:2 all-to-all group. omnidiff maps it to the first
  and inserts the second (`option`, `.`, `getArgName`, `(`, `)`). This is the fixture's N:M floor
  (5 visible). `nm-floor`.
- **`option.hasArgName()` on line 653 (10 visible, 4 invisible).** The ground truth deletes it and
  inserts the first new `option.getArgName()` of the condition. omnidiff pairs them, reason
  `APTED("large_flat_subtree")`: `option`, `.`, `()` Identical and `hasArgName` -> `getArgName`
  as an Update. The receiver is the same and the method name changes by two letters, so a rename
  reading is defensible; the ground truth's reading (a predicate call replaced by an accessor
  call) is too. `ambiguous`.

## Painting

All of `real` is the matcher's (absent with the human mapping):

- N:M, Minimal 59 / Full 58 bytes: on line 656 both paintings paint the before call and its first
  copy `Move`, which omnidiff's one-to-one in-place pair does not; Minimal leaves the second
  copy unpainted where omnidiff paints it Insert, Full paints it `Move`. `nm-floor`.
- Minimal 39 / Full 39 bytes, Move/Update -> Delete/Insert: `option.hasArgName()` (before line
  653) and `option.getArgName() ` (after line 653), painted as a kept receiver with a renamed
  method instead of a deletion and an insertion. `ambiguous`.

With the human mapping the renderer is off by 3 single spaces under Full (after line 656), not in
`real`.

## Verdict

- Primary cause: `nm-floor`
- Other causes: `ambiguous`
- Where a fix would go: N:M support (`solve_multi_maps`) for a call duplicated into a ternary;
  none for the line-653 pairing (ambiguous)
- Ground truth questionable: no (Marko marks the solution as possibly controversial himself)
