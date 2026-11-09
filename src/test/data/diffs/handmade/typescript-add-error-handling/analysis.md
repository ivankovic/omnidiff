# Accuracy analysis: typescript-add-error-handling

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 185 nodes, 185 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 410: renderer 0, matcher 0 |
| Painting, Full | 8 bytes wrong of 410: renderer 8, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A handmade fixture: `divide` gains an `if (b === 0) { throw ... }` guard, and the two top-level
statements `const result = divide(10, 2);` and `console.log(...)` are wrapped in a
`try { ... } catch (error) { console.error(...) }` (after lines 8-13), so they are indented by four
spaces. The mapping is perfect and Minimal agrees byte for byte.

## Painting

### Full, 8 bytes (renderer; the same with the human mapping)

The Full painting marks the new four-space indentation of the two wrapped lines (after lines 9
and 10) as Insert and the statements themselves as moved (Match).

- After line 9, `    `: None -> Insert. omnidiff leaves it unpainted; it precedes the first moved
  statement and follows the inserted `try {`.
- After line 10, `    `: Move -> Insert. It lies between the two moved statements, and omnidiff
  paints it Move with them (gap text between adjacent moved nodes).

`renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` (Full): indentation added by a new wrapper should be painted
  Insert, not left unpainted or absorbed into the Move gap between moved siblings.
- Ground truth questionable: no
