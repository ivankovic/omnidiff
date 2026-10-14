# Accuracy analysis: java-defects4j-closure-117-typevalidator

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 10808 nodes, 10808 graded, N:M floor 0 visible |
| Painting, Minimal | 122 bytes wrong of 62582: renderer 122, matcher 0 |
| Painting, Full | 16 bytes wrong of 62582: renderer 16, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-117: `TypeValidator.getReadableJSTypeName` computes and dereferences the node's type
first: two statements, the `JSType type = getJSType(n); if (dereference) {...}` block and the
`if (type.isFunctionPrototypeType() || ...) { return type.toString(); }` check (before lines
756-767), move to the top of the method (after lines 725-731 and 734-738).

## Painting

Mapping is exact. All disagreement survives with the human mapping, so it is renderer-owned.

- Minimal, Move -> None, 122 bytes: the indentation of every line after the first inside the two
  moved multi-line statements (before lines 757-762 and 764-767, after 726-731 and 735-738). The
  renderer paints each moved statement as one multi-line Move range and Minimal's whitespace
  trimming only trims the ends of a range, so the interior indentation stays painted. The Minimal
  painting paints each line's content separately and leaves all indentation alone. `renderer`.
- Full, None -> Move, 16 bytes: the first-line indentation of each moved statement (before lines
  756 and 763, after 725 and 734, 4 bytes each), which the Full painting carries with the move
  and omnidiff leaves unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` (`ranges_for_options`): Minimal should drop leading
  indentation on every row of a multi-row range, not only at the range's ends; Full should
  include a moved statement's first-line indentation.
- Ground truth questionable: no.
