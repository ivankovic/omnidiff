# Accuracy analysis: c-neovim-neovim-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 68 nodes, 68 graded, N:M floor 0 visible |
| Painting, Minimal | 64 bytes wrong of 308: renderer 64, matcher 0 |
| Painting, Full | 182 bytes wrong of 308: renderer 182, matcher 0 |
| Ground-truth invariant violations | none |

## The change

neovim `src/nvim/eval/gc.h` (8 lines): the two `extern` declarations (before lines 5-6) move
below `#include "eval/gc.h.generated.h"` (after lines 7-8) and gain a `DLLEXPORT ` prefix.

## Painting

The mapping has no mismatches; all runs are the same with the human mapping.

- **None -> Move, 64 bytes, both presets:** `#include "eval/gc.h.generated.h"` (before line 8, after
  line 5). Both paintings record the include as the line that moved (one line past two).
  omnidiff does not paint it: under Minimal it shows no reorder at all (the two `extern` lines
  are updated, and the unchanged part of an Update is not painted Move); under Full it paints the
  two `extern` lines Move instead, the larger of the two accounts. `renderer`.
- **Full, Move -> Update and None -> Update, 118 bytes:** the `extern ...;` text of both
  declarations. The Full painting's `Match` spans are `extern dict_T *gc_first_dict;` (before)
  against ` extern dict_T *gc_first_dict;` (after, from column 9, including the space after
  `DLLEXPORT`); the extra space makes the pair an Update rather than a Move. omnidiff paints the
  before side Move (58 bytes) and leaves the after side unpainted at its first byte (60 bytes). The
  one-space offset looks like a selection slip in the painting. `ground-truth`.

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth`
- Where a fix would go: `diff::text` move attribution (`reconcile_moves`) when the reordered
  lines are also updated; the Full painting's span offset is Marko's call
- Ground truth questionable: yes: the Full painting's after-side spans start one column early (on
  the space after `DLLEXPORT`), turning the moved declarations into Updates
