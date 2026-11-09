# Accuracy analysis: c-freeciv-freeciv-null-to-nullptr

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 18 (18 visible) of 28224 nodes, 28224 graded, N:M floor 0 visible |
| Painting, Minimal | 190 bytes wrong of 180217: renderer 16, matcher 190 |
| Painting, Full | 684 bytes wrong of 180217: renderer 498, matcher 190 |
| Ground-truth invariant violations | none |

## The change

freeciv `common/aicore/cm.c` (~90 KB): every `NULL` becomes `nullptr` (16 places), the two Yoda
conditions `NULL != pwork && ...` become `pwork != nullptr && ...` (lines 2288, 2445), two
comments are capitalised and one comment line is split in two (before line 568).

## Mapping

- **`NULL` -> `nullptr`, 14 visible.** The ground truth updates each `NULL` token to `nullptr`
  (`MatchButNotIdentical`). omnidiff deletes and inserts each one (`APTED("qualified_name")`):
  in tree-sitter-c the two are different token kinds (`NULL` vs `nullptr` under `null`), and
  omnidiff's tree edit distance does not relabel across kinds. `matcher`.
- **`NULL != pwork` -> `pwork != nullptr`, 4 visible** (before lines 2288, 2445). Ground truth
  keeps `!=` and `pwork` (a swap of the operands). omnidiff deletes both, so with `NULL`
  unmappable the whole comparison is delete + insert. An ordered mapping can keep only one of the
  crossing pairs `pwork`/`!=`; omnidiff keeps neither (`APTED("qualified_name")`). `matcher`.

## Painting

- **Matcher, 174 bytes Minimal / 186 Full:** each `NULL` painted Delete and each `nullptr` Insert
  where the painting has Update, and the two swapped comparisons (`Delete/Insert -> Update/Move`).
  Rendering the human mapping agrees with the painting on all of them. `matcher`.
- **Full, Move -> None, 462 bytes (also with the human mapping):** after each operand swap, the
  rest of the statement is painted Move: `&& pwork == pcity);` on before/after lines 2288-2289,
  and on lines 2445-2452 the `&& pwork == pcity) {` tail plus the whole `if` body
  (`int cx, cy;`, `city_tile_index_to_xy(...)`, `log_test(...)`, `}`). The painting leaves them
  alone. The crossed pair makes the Full walk flag everything after it in the statement as out
  of order. `renderer`.
- **Renderer, 16 bytes Minimal / 36 Full (shared):** the swapped `pwork != nullptr` rendered with
  `pwork` not painted as the painting's Move on after lines 2289, 2446 (Minimal), and the 2-byte
  indentation of the split comment line (before 568 / after 569) that the Full painting moves
  (Full). `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: the APTED kind barrier (`apted::common::resolve`): allow `NULL` <->
  `nullptr` (same parent kind `null`) as an Update; Full move attribution after an operand swap in
  `diff::text::reconcile_moves`
- Ground truth questionable: no
