# Accuracy analysis: java-defects4j-jacksondatabind-103-deserializercache

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 6439 nodes, 6439 graded, N:M floor 0 visible |
| Painting, Minimal | 17 bytes wrong of 50745: renderer 2, matcher 15 |
| Painting, Full | 17 bytes wrong of 50745: renderer 2, matcher 15 |
| Ground-truth invariant violations | none |

## The change

Jackson-databind 103 replaces `iae.getMessage()` with `ClassUtil.exceptionMessage(iae)` across the
codebase; in this file the change is on line 268.

## Mapping

- Line 268, the receiver `iae` of `iae.getMessage()` and the new receiver `ClassUtil`: the ground
  truth moves `iae` into the argument list of `exceptionMessage(..)` and inserts `ClassUtil`.
  omnidiff (`APTED("qualified_name")`) keeps the receiver slot: `iae` -> `ClassUtil` as an Update,
  and the argument `iae` inserted. 2 visible. The edit-distance fallback pairs the two receivers by
  position; the value `iae` itself survives as the argument, which is the ground truth's reading.
  `matcher`.

## Painting

Matcher-caused (absent with the human mapping):

- Both presets: before `iae` (with its `.` in Minimal) painted Update where the painting has Move,
  after `ClassUtil` painted Update where it has Insert, and the after argument `iae` painted Insert
  where it has Move. Minimal 15 bytes, Full 15 bytes. `matcher`.

Renderer-caused (also with the human mapping):

- Minimal, 2 bytes: the human mapping, rendered, paints the shifted `.` Move on both sides; the
  Minimal painting leaves it unpainted (the Full painting marks it Move). In omnidiff's own
  rendering this `.` sits inside the matcher runs above. `renderer`.
- Full, `None -> Move`, 2 bytes: the Full painting marks the argument list's `)` Move on both sides;
  omnidiff leaves it unpainted because `)` lies in the identical tail of the row (the suffix case of
  `node_untouched_on_its_row`), though it changed role from closing `()` to closing `(iae)`.
  `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: APTED fallback (qualified_name): a receiver that becomes the argument of a
  static helper call
- Ground truth questionable: no
