# Accuracy analysis: cpp-add-memory-management

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 196 nodes, 196 graded, N:M floor 0 visible |
| Painting, Minimal | 42 bytes wrong of 540: renderer 42, matcher 0 |
| Painting, Full | 11 bytes wrong of 540: renderer 11, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Handmade C++ fixture (no provenance README): `#include <memory>` is added (after line 2) and the
raw pointer `Resource* res = new Resource();` (before line 10) becomes
`std::unique_ptr<Resource> res = std::make_unique<Resource>();` (after line 11).

## Painting

The mapping is perfect and every run is identical with the human mapping: all renderer.

- Minimal, `Move -> None`, 42 bytes, the two `Resource` type identifiers and `res =` on before line
  10 / after line 11: these tokens are kept but reparented (into the template arguments of
  `unique_ptr` and `make_unique`) and land in new columns. omnidiff paints them Move. The Minimal
  painting marks only the inserted `std::unique_ptr<`, `>`, `std::make_unique<`, `>` and the
  deleted `*` and `new`, and leaves the kept tokens unpainted. `renderer`.
- Full, 11 bytes on the same lines: the Full painting matches ` res = ` with its surrounding
  spaces as one Move and deletes the space after `new`; omnidiff paints only the `res =` tokens
  Move and leaves the 4 spaces and the deleted space unpainted (5 bytes). omnidiff also paints the
  trailing `();` Move on both sides (6 bytes), where the painting leaves the identical tail
  unpainted (painting rule 3). `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `identical_or_move` in `src/diff/text.rs` (Minimal: kept tokens
  reparented within a rewritten line; Full: displaced identical tail) and Full gap-whitespace
  painting
- Ground truth questionable: no
