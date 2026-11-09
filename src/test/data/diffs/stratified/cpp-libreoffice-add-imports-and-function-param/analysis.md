# Accuracy analysis: cpp-libreoffice-add-imports-and-function-param

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 275 nodes, 275 graded, N:M floor 0 visible |
| Painting, Minimal | 83 bytes wrong of 3286: renderer 83, matcher 0 |
| Painting, Full | 86 bytes wrong of 3286: renderer 86, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The commit adds two `#include`s (`<hb.h>`, `<vector>`). `SyncPattern` gains a parameter
`const std::vector<hb_variation_t>& rFontVariations`, and its one-line declaration (line 33) is
rewrapped over three lines (after lines 35-37).

## Painting

- **Rewrapped parameters painted Move (both presets, `Move -> None`, `renderer`).** The parameters
  `sal_uInt32 nFontVariation, bool bEmbolden` (before line 33, after line 36) are unchanged, but a
  line break now comes before them. Omnidiff paints them Move on both sides: 41 + 42 bytes in
  Minimal, and with `);` (after line 37) 86 bytes in Full. Both paintings leave them unpainted:
  rewrapping is not a move. The runs are identical with the human mapping.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: move display in `src/diff/text.rs`. Siblings that kept their order inside
  the same parent would not be painted Move when only a line break was inserted before them.
- Ground truth questionable: no
