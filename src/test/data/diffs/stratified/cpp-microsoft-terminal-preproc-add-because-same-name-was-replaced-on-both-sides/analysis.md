# Accuracy analysis: cpp-microsoft-terminal-preproc-add-because-same-name-was-replaced-on-both-sides

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 11248 nodes, 11248 graded, N:M floor 0 visible |
| Painting, Minimal | 29 bytes wrong of 56235: renderer 2, matcher 27 |
| Painting, Full | 33 bytes wrong of 56235: renderer 6, matcher 28 |
| Ground-truth invariant violations | 1 |

## The change

The guard `#if !defined(_HAS_CXX23) || !_HAS_CXX23` becomes
`#if !defined(__cpp_lib_string_resize_and_overwrite) && _MSVC_STL_UPDATE >= 202111L` (line 570). An
`#elif ... #error` branch is added (after lines 572-573). The macro `_HAS_CXX23` is replaced in both
places it occurred.

## Mapping

- **Unrelated identifiers paired (2 visible, `matcher`, `APTED("qualified_name")`).** The ground
  truth treats the first `_HAS_CXX23` (inside `defined(...)`) as renamed to
  `__cpp_lib_string_resize_and_overwrite`. It deletes the second, the operand of `!_HAS_CXX23`, and
  inserts `_MSVC_STL_UPDATE`, the left operand of the new `>= 202111L` comparison. Omnidiff updates
  the second `_HAS_CXX23` into `_MSVC_STL_UPDATE`. The two sit in different expressions (a unary `!`
  operand vs a comparison operand) and share no text. The rename is cheap for APTED, but a reader
  sees a rename that did not happen.

## Painting

- **Matcher (Minimal 27, Full 28 bytes).** `_HAS_CXX23` and `_MSVC_STL_UPDATE ` are rendered as the
  two sides of an Update. The paintings have a Delete and an Insert. The Full painting labels the
  after side Move, as part of a larger identical span.
- **Ground truth (Minimal, 2 bytes, `None -> Update`).** The Minimal painting includes the shared
  leading `_` in the `_HAS_CXX23` -> `__cpp_lib_...` rename on both sides. Omnidiff narrows it out.
  `invariants.txt` flags this ([16]).
- **Renderer (Full, 6 bytes, `None -> Move`).** The `(` of `defined(` and single spaces around the
  changed operands are marked Move by the Full painting. Omnidiff leaves them unpainted. These runs
  are the same with the human mapping.

## Verdict

- Primary cause: `matcher`
- Other causes: `ground-truth`, `renderer`
- Where a fix would go: APTED's rename cost in the `qualified_name` call. Identifiers in different
  operator roles, with no shared text, should not pair as an Update.
- Ground truth questionable: yes. The Minimal painting paints the shared leading `_` of a renamed
  identifier, against its own word-level convention (invariant [16]).
