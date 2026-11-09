# Accuracy analysis: go-gin-gonic-gin-enclose-with-var

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 6 (3 visible) of 865 nodes, 867 graded, N:M floor 1 visible |
| Painting, Minimal | 61 bytes wrong of 4917: renderer 53, matcher 12 |
| Painting, Full | 102 bytes wrong of 4917: renderer 94, matcher 10 |
| Ground-truth invariant violations | none |

## The change

In gin's `mode.go`, two single declarations (lines 47-48) `var ginMode int32 = debugCode` and
`var modeName atomic.Value` become one grouped `var ( ... )` block (after lines 47-50). gofmt
widens the alignment, so the first spec becomes `ginMode  int32` with two spaces.

## Mapping

- Lines 47-48 (before) and line 47 (after): 2 visible and 2 invisible mismatches are the
  `matcher`'s. The ground truth maps both before `var_declaration`s, and both `var` keywords, 2:1
  onto the after declaration and its `var`. Omnidiff's `APTED("fast_fallback")` deletes both
  before declarations and inserts the after one. No `var` keyword survives, although the specs
  inside are matched.
- 1 visible and 1 invisible mismatch are the N:M floor (`vis_nm_floor` 1). With a one-to-one
  mapping, only one of the two before declarations can be paired. `nm-floor`.

## Painting

The two paintings agree. Each moves `var`,`var` -> `var` (2:1), inserts `(` and `)` (and in Full
also the space before `(`), and leaves both specs unpainted.

- Renderer, Minimal 51 bytes, Full 93 bytes, `Move -> None`. These runs are also present with the
  human mapping. The `ginMode` spec is painted Move on both sides: it moved from column 4 to
  column 1 under a new parent, and its whitespace changed. Full also paints the byte-identical
  `modeName` spec Move. The painting treats the specs as unchanged. `renderer`.
- Matcher, Minimal 10 bytes, Full 9 bytes, `Delete/Insert -> Move`. With omnidiff's mapping, the
  `var` keywords are painted Delete (before lines 47, 48) and Insert (after line 47) instead of the
  painting's Move. A side effect: because omnidiff inserts the whole after declaration, its `(`
  and `)` happen to be painted Insert as the painting wants. With the human mapping, Minimal drops
  them as lone brackets (`None -> Insert`, 2 bytes, renderer). `matcher`.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`, `nm-floor`
- Where a fix would go: the Move painting of specs that only moved into a new grouped
  declaration (`src/diff/text.rs`, `identical_or_move`). Also the `apted` fast_fallback, which
  should pair a single declaration with the grouped block that replaces it.
- Ground truth questionable: no
