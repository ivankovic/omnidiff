# Accuracy analysis: c-ollama-ollama-change-imports

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 8 (4 visible) of 422 nodes, 422 graded, N:M floor 0 visible |
| Painting, Minimal | 36 bytes wrong of 1450: renderer 0, matcher 36 |
| Painting, Full | 36 bytes wrong of 1450: renderer 0, matcher 36 |
| Ground-truth invariant violations | none |

One painting (`Only one solution`), scored under both presets.

## The change

ollama's vendored `json-schema-to-grammar.h`: the header block on lines 3-6 (`#include "ggml.h"`,
a comment, `#define JSON_ASSERT GGML_ASSERT`, `#include "json.hpp"`) is replaced by
`#include <nlohmann/json_fwd.hpp>`, a blank line, `#include <functional>` and `#include <string>`.
The rest of the file is unchanged.

## Mapping

- Lines 3 and 6, both sides, 4 visible + 4 invisible: the ground truth deletes both old
  `preproc_include`s and inserts all three new ones, with nothing mapped between them. omnidiff
  pairs them by position, all eight through `APTED("fast_fallback")` (the root-level
  tree-edit-distance fallback `apted::for_roots_fallback`, run after the anchored passes):
  `#include "ggml.h"` (line 3) with `#include <nlohmann/json_fwd.hpp>` (line 3) and
  `#include "json.hpp"` (line 6) with `#include <string>` (line 6). The visible
  mismatches are the `#include` keyword tokens of these two pairs; the invisible ones are the
  `preproc_include` nodes. The paired includes have nothing in common but the keyword, and
  the new headers are different headers, not renamed ones. The ground truth's reading is the
  reasonable one. `matcher`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper
began printing them.

## Painting

- Both presets, `None -> Delete` / `None -> Insert`, line 3 and line 6 on both sides, 36 bytes
  (4 x `#include `): because omnidiff pairs the includes, it paints only the changed path as an
  update and leaves the `#include ` keyword and space unpainted; the painting deletes and inserts
  the whole lines. Absent with the human mapping, so it is the matcher's. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the root fallback `apted::for_roots_fallback` (`"fast_fallback"`,
  src/diff.rs), which pairs same-kind `preproc_include` nodes by position when only the keyword
  is shared
- Ground truth questionable: no
