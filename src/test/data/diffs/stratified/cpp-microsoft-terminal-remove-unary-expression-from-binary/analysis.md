# Accuracy analysis: cpp-microsoft-terminal-remove-unary-expression-from-binary

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 7331 nodes, 7331 graded, N:M floor 0 visible |
| Painting, Minimal | 1 byte wrong of 34655: renderer 1, matcher 0 |
| Painting, Full | 1 byte wrong of 34655: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`return FirstWindowPreference() == ...::PersistedWindowLayout && !IsolatedMode();` loses its
`&& !IsolatedMode()` operand (line 370).

## Painting

Both runs are the same with the human mapping.

- **Minimal, 1 byte (`Delete -> None`, `renderer`).** Omnidiff paints the deletion as one range
  `&& !IsolatedMode()`, including the space between `&&` and `!`. The Minimal painting has `&&` and
  `!IsolatedMode()` as two ranges and leaves that space out.
- **Full, 1 byte (`None -> Delete`, `renderer`).** The Full painting also deletes the space before
  `&&`. Omnidiff's Full rendering leaves it unpainted.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: whitespace handling around a deleted operand in `src/diff/text.rs`. These
  are one-byte differences, and the painting's split in Minimal is a matter of taste.
- Ground truth questionable: no
