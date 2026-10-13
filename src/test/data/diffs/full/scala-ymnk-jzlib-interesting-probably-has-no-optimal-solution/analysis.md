# Accuracy analysis: scala-ymnk-jzlib-interesting-probably-has-no-optimal-solution

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 457 nodes, 457 graded, N:M floor 0 visible |
| Painting, Minimal | 3 bytes wrong of 2036: renderer 3, matcher 0 |
| Painting, Full | 50 bytes wrong of 2036: renderer 50, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In the sbt build, `val scalatest = "org.scalatest" %% "scalatest" % "1.6.1"` (line 25) becomes
`"org.scalatest" % "scalatest_2.10" % "1.9.1" % "test"`, the `junit` and `junit_interface` vals
are deleted and dropped from the dependency list. The mapping is perfect; all disagreement is on
line 25 and is the same with the human mapping, so it is the renderer's.

## Painting

### Minimal, 3 bytes

- `%%` -> `%` (before byte 532, the second `%`): Update -> Delete. The two operator tokens are
  mapped as an Update and omnidiff narrows it to the differing character, painted Update; the
  painting deletes the extra `%`. `renderer`.
- The `%` before the version string, before column 49 and after column 53 (1 byte each side):
  Move -> None. The token is unchanged; it sits in a left-associative infix chain that gained a
  level when `% "test"` was appended, and its column shifted by the inserted `_2.10`. omnidiff
  paints it Move, the painting leaves it alone. Which of the two triggers the Move was not
  determined. `renderer`.

### Full, 50 bytes

None -> Update over `%% "scalatest" % "1.6.1"` (22 bytes before) and
`% "scalatest_2.10" % "1.9.1" ` (28 bytes after). The Full painting records the whole tail as one
Update (`%%` -> `%`, then ` "scalatest" % "1.6.1"` -> ` "scalatest_2.10" % "1.9.1"`); omnidiff
paints only the changed characters (the second `%`, `_2.10` as Insert, `6` -> `9`) and leaves the
unchanged ones in between unpainted. `renderer` (update narrowing vs a wide Full span).

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text`: Delete instead of Update for the surplus character of a
  narrowed operator update (`%%` -> `%`), no Move for an operator re-nested in a growing infix
  chain; for Full, update narrowing inside a changed operand run.
- Ground truth questionable: no
