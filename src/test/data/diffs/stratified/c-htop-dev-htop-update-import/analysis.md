# Accuracy analysis: c-htop-dev-htop-update-import

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 438 nodes, 438 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 2567: renderer 0, matcher 0 |
| Painting, Full | 18 bytes wrong of 2567: renderer 18, matcher 0 |
| Ground-truth invariant violations | none |

## The change

htop header: `#include "ProcessList.h"` becomes `#include "ProcessTable.h"` (line 22).

## Painting

- **Full, None -> Update, 18 bytes:** `Process` and `.h` on line 22, both sides. The Full painting
  updates the whole path `ProcessList.h` -> `ProcessTable.h`; omnidiff's Full rendering narrows
  the Update to `List` -> `Table`, as Minimal does (and as the Minimal painting does). The same
  with the human mapping. `renderer`.

Minimal agrees with the painting; the mapping has no mismatches.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` Full rendering of an updated string/path: paint the whole
  token, not the narrowed word
- Ground truth questionable: no
