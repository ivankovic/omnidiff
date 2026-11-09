# Accuracy analysis: c-genymobile-scrcpy-add-to-import-path-and-move-imports-around

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 210 nodes, 210 graded, N:M floor 0 visible |
| Painting, Minimal | 18 bytes wrong of 1020: renderer 18, matcher 0 |
| Painting, Full | 38 bytes wrong of 1020: renderer 38, matcher 0 |
| Ground-truth invariant violations | none |

## The change

scrcpy `app/src/usb/keyboard_aoa.h`: `#include "aoa_hid.h"` (line 8) becomes
`#include "usb/aoa_hid.h"` and swaps places with `#include "hid/hid_keyboard.h"` (now line 9).

## Painting

The mapping has no mismatches; all runs are the same with the human mapping.

- **Minimal, None -> Move, 18 bytes:** `aoa_hid.h` on before line 8 / after line 9. Both paintings
  record the `aoa_hid` include as the line that moved (plus `usb/` inserted). omnidiff paints
  only the inserted `usb/` and does not show the reorder on either line: the moved include is
  also updated, and the unchanged part of an Update is not painted Move. `renderer`.
- **Full, None -> Move, 38 bytes:** the same, plus `#include "` on both sides, which the Full
  painting includes in the Move. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text`: paint the unchanged part of an updated node Move when the
  node also crossed a sibling (`crossed_backwards` / `reconcile_moves`)
- Ground truth questionable: no
