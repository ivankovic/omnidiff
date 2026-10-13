# Accuracy analysis: java-defects4j-mockito-32-spyannotationengine

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 10 (8 visible) of 1264 nodes, 1264 graded, N:M floor 0 visible |
| Painting, Minimal | 27 bytes wrong of 6035: renderer 5, matcher 28 |
| Painting, Full | 28 bytes wrong of 6035: renderer 36, matcher 56 |
| Ground-truth invariant violations | none |

## The change

Defects4J Mockito-32. `SpyAnnotationEngine` stops creating spies with `Mockito.spy(instance)`. It
uses `Mockito.mock(instance.getClass(), withSettings().spiedInstance(instance)
.defaultAnswer(Mockito.CALLS_REAL_METHODS).name(field.getName()))`, so the spy gets the field's
name. A static import of `withSettings` is added. The file has CRLF line endings.

## Mapping

- **Which call `spy(instance)` became (8 visible, 2 invisible), `ambiguous`.** The ground truth maps
  `.spy(instance)` on line 49 to `.spiedInstance(instance)` on line 51: `spy` -> `spiedInstance`
  (Update), and `.`, `(instance)` with its parens. The new `.mock(` is inserted. omnidiff
  (`APTED("qualified_name")`) keeps the outer call: `spy` -> `mock` (Update), `.` and `(` -> those
  of `.mock(` on line 50, and the old `)` -> the final `)` of the mock call on line
  53. Its argument list `(instance)` is MatchButNotIdentical with the new four-line list. Both
      readings are faithful. The ground truth follows the name and the identical `(instance)`
      argument. omnidiff follows the call that still receives the result, `Mockito.X(...)`. Neither
      is wrong.

## Painting

- **Matcher-owned, 24 bytes per preset.** `.`, `(`, `)` on line 49, `.mock(` on line 50,
  `.spiedInstance(` and `)` on line 51 and `))` on line 53 follow omnidiff's reading instead of the
  painting's. `ambiguous`.
- **Renderer, Minimal 5 bytes.** Rendering the human mapping's `spy` -> `spiedInstance` Update
  leaves the shared `sp` unpainted. The painting updates `spy` whole. `renderer`.
- **Renderer, Full 36 bytes.** The Full painting inserts the 32-column indentation of the
  `.spiedInstance(instance)` continuation line (line 51). Rendering the human mapping leaves it
  unpainted, because the row carries a matched call. The trailing `);` on line 49 is painted Move.
  `renderer`.

## Verdict

- Primary cause: `ambiguous`
- Other causes: `renderer`
- Where a fix would go: none for the mapping (ambiguous); `src/diff/text.rs` for the rename split
  keeping a shared prefix (Minimal) and the indentation of a continuation line that holds a matched
  call (Full)
- Ground truth questionable: no
