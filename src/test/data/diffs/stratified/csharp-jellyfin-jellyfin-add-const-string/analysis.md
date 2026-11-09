# Accuracy analysis: csharp-jellyfin-jellyfin-add-const-string

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 131730 nodes, 131730 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 685182: renderer 2, matcher 0 |
| Painting, Full | 2 bytes wrong of 685182: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In Jellyfin's `EncodingHelper.cs`, one string concatenation (line 6555) gains a constant:
`... : string.Empty) + (isAv1 ? ...)` becomes `... : string.Empty) + " -threads 2" + (isAv1 ? ...)`.

## Painting

There is one painting ("Only one solution"), scored under both presets. It inserts `+`, the space
and `" -threads 2"`. The 2 bytes are the same in both presets and with the human mapping, so they
are the renderer's.

- `Move -> None`, line 6555, 1 byte per side: the existing `+` before `(isAv1 ? ...)`. `+` nests to
  the left, so the insert wraps the old left operand in a new `binary_expression`. The outer `+`
  keeps its parent but shifts right on the row. The insert is inside its sibling's subtree, not
  beside it, and the renderer paints the lone `+` grey Move. The painting leaves it unpainted.
  `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `identical_or_move` in `src/diff/text.rs`. A token shifted on its row only
  by an insert inside a sibling's subtree should stay Identical.
- Ground truth questionable: no
