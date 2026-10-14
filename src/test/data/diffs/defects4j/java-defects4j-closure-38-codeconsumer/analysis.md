# Accuracy analysis: java-defects4j-closure-38-codeconsumer

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 2688 nodes, 2688 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 14136: renderer 0, matcher 0 |
| Painting, Full | 10 bytes wrong of 14136: renderer 10, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Line 245: `if (x < 0 && prev == '-')` becomes `if ((x < 0 || negativeZero) && prev == '-')`, so
negative zero also gets a separating space.

## Painting

The fixture has a single painting (`Only one solution`), which inserts `(` and `|| negativeZero)`
and leaves `x < 0` unpainted. Minimal matches it exactly.

Full, `Move -> None`, line 245, 5 bytes each side (`x < 0`): omnidiff's Full rendering paints the
comparison Move because it went one level deeper (into the new parenthesized `||`) and shifted right
by one column. The run is the same with the human mapping. Painting a re-parented, shifted node Move
is what Full does on purpose (the Full painting of
`java-defects4j-closure-30-flowsensitiveinlinevariables` marks a shifted `root)` Move in the same
way), so this disagreement exists only because Full is scored against a Minimal-style single
painting. `grading-artifact`.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: none
- Where a fix would go: none: Full held to a single Minimal-style painting
- Ground truth questionable: no
