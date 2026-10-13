# Accuracy analysis: kotlin-refactor-function

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 74 (54 visible) of 265 nodes, 274 graded, N:M floor 7 visible |
| Painting, Minimal | 362 bytes wrong of 698: renderer 236, matcher 366 |
| Painting, Full | 378 bytes wrong of 698: renderer 170, matcher 361 |
| Ground-truth invariant violations | none |

## The change

Handmade fixture. The top-level functions `calculateRectangleArea(width, height)` and
`calculateRectanglePerimeter(width, height)` become the members `area()` and `perimeter()` of
a new `data class Rectangle(val width: Double, val height: Double)`, with expression bodies.
In `main`, `val width = 5.0` / `val height = 3.0` become `val rectangle = Rectangle(5.0, 3.0)`,
and the calls become `rectangle.area()` / `rectangle.perimeter()`. description.md: "Requires
N:M mapping".

## Mapping

- **Functions moved into the class and renamed** (22 visible, 12 invisible, reason
  `APTED("fast_fallback")`). The ground truth maps each top-level function to its member
  (names updated, `fun`, `:`, `Double`, the body `width * height` / `2 * (width + height)` kept).
  Omnidiff deletes both functions and inserts both members. Their qualified names differ
  (`calculateRectangleArea` vs `Rectangle::area`), so `solve_qualified_name_groups` does not pair
  them, and the residual LCS fallback does not either. `matcher`: rename plus move into a class
  not detected.
- **Parameter lists, 2:1** (21 visible, 6 invisible, `APTED("fast_fallback")`). The ground truth
  maps both `width: Double, height: Double` lists all-to-all onto the one constructor list
  `val width: Double, val height: Double`. Omnidiff deletes both and inserts the constructor
  parameters. A one-to-one mapping can keep only one of the two before copies, so 7 visible
  mismatches are the N:M floor; the other 14 are the matcher's (it keeps neither). `nm-floor` and
  `matcher`.
- **`main`** (11 visible, 2 invisible, reason `APTED("qualified_name")` on the `main` match).
  The ground truth deletes `val width =` and `val height =`, inserts `val rectangle =`, moves `5.0`
  and `3.0` into `Rectangle(5.0, 3.0)`, and updates `calculateRectangleArea` ->
  `area`, `calculateRectanglePerimeter` -> `perimeter` (inserting the `rectangle.` receiver).
  Omnidiff pairs by position: `val width = ` -> `val rectangle = ` (`width` -> `rectangle`
  update), `3.0` deleted, and `calculateRectangleArea` -> `rectangle` (the first identifier of
  `rectangle.area()`), so `area` is new. The ground truth follows the renamed functions; APTED
  takes the leftmost leaf. `matcher`.

## Painting

The two mappings differ almost everywhere, so most wrong bytes have a matcher share; a large part
is also wrong with the human mapping.

- Matcher (Minimal about 139 bytes wrong only with omnidiff's mapping, Full about 222): the
  deleted top-level functions (before lines 1-7) and the inserted `fun area(): Double` / `fun
  perimeter(): Double` (after lines 2-3) where the paintings have updates and moves;
  `val width =` left unpainted and `val rectangle =` unpainted (Update) where the paintings have
  Delete and Insert; `rectangle`/`area` and `rectangle`/`perimeter` with swapped Update/Insert
  labels (after lines 8-9); `3.0` deleted and inserted instead of moved. `matcher`.
- Wrong with the human mapping too (Minimal about 223 bytes of the real total, Full about 156):
  the 2:1 parameter lists, which the paintings paint Update and the rendering paints Move (before)
  or Insert (after, the second copy has no 1:1 partner); `fun` painted Move where the paintings
  leave it; the function bodies, which the paintings paint Move and the rendering leaves
  unpainted; name narrowing (`rea`, `erimeter`) of the renamed functions; under Full also
  indentation and the closing `}")` of the string templates (painted Move because they shifted).
  Mostly `nm-floor` (the 2:1 lists, about 80 bytes per preset on both sides) and `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `nm-floor`, `renderer`
- Where a fix would go: matching renamed functions that move into a new class (an identity signal
  beyond qualified name, e.g. body similarity, before the residual fallback); APTED's leftmost
  pairing of `calculateRectangleArea` -> `rectangle` in `main`; renderer Move/Update labels for
  N:M groups.
- Ground truth questionable: no
