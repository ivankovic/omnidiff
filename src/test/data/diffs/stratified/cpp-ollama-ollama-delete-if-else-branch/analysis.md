# Accuracy analysis: cpp-ollama-ollama-delete-if-else-branch

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1976 nodes, 1976 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 7563: renderer 0, matcher 0 |
| Painting, Full | 80 bytes wrong of 7563: renderer 80, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The `else if (proj_type == PROJECTOR_TYPE_MUSIC_FLAMINGO) { ... build_ffn(...) ... }` branch is
deleted from an if-else chain (before lines 89-97). It is a pure deletion.

## Painting

- **Indentation of the deleted branch (Full, 80 bytes, `Delete -> None`, `grading-artifact`).** Full
  paints the leading spaces of the deleted lines (before lines 90-96) and the indentation before the
  branch's closing `}` (line 98) Delete. The fixture has only a `Minimal` painting, which leaves
  indentation unpainted. The runs are the same with the human mapping.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: none
- Where a fix would go: none. Full is scored against a Minimal-only painting.
- Ground truth questionable: no
