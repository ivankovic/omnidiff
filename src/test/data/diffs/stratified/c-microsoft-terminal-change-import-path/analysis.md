# Accuracy analysis: c-microsoft-terminal-change-import-path

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 272 nodes, 272 graded, N:M floor 0 visible |
| Painting, Minimal | 8 bytes wrong of 3951: renderer 8, matcher 0 |
| Painting, Full | 17 bytes wrong of 3951: renderer 17, matcher 0 |
| Ground-truth invariant violations | none |

## The change

microsoft/terminal header: `#include <json.h>` becomes `#include <json/json.h>` (line 54).

## Painting

The mapping has no mismatches; all runs are the same with the human mapping.

- **Minimal, 8 bytes:** which `json` is new. The Minimal painting inserts the first `json` and
  the `/` (`json/` + old `json.h`); omnidiff inserts `/json` after the old `json` (old `json` +
  `/json` + `.h`). Both splits describe the same text change; the painting's reading (a directory
  prefix added) is the one the commit means, but nothing in the text decides it. `ambiguous`.
- **Full, None -> Update, 17 bytes:** `json.h` / `json/json.h`. The Full painting updates the
  whole path; omnidiff's Full rendering narrows to the inserted part and leaves the rest
  unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `ambiguous`
- Where a fix would go: `diff::text` Full rendering of an updated path (whole token); the Minimal
  split is none (ambiguous)
- Ground truth questionable: no
