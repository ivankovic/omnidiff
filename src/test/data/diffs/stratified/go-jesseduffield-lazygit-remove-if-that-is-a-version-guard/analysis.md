# Accuracy analysis: go-jesseduffield-lazygit-remove-if-that-is-a-version-guard

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 2544 nodes, 2544 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 7146: renderer 0, matcher 0 |
| Painting, Full | 7 bytes wrong of 7146: renderer 7, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A version guard is removed in an integration test.
`if t.Git().Version().IsAtLeast(2, 22, 0) { A } else { B }` (before lines 34-38) becomes just `A`
(after line 34), one indentation level shallower.

## Painting

Minimal matches exactly. The two Full runs are the same with the human mapping, so they are the
renderer's.

- Full, `None -> Delete` (4 bytes) and `None -> Move` (3 bytes). These are the indentation of the
  kept `t.Views().Status().Content(...)` line: before line 35 has four tabs, after line 34 has
  three. The Full painting deletes one tab and moves the other three with the statement. Omnidiff's
  Full rendering does not paint the reindented statement's leading whitespace. The statement is
  left unpainted, as the painting wants, but its indentation is unpainted too. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full rendering of a dedented statement's indentation (`src/diff/text.rs`,
  leading-whitespace handling for statements unwrapped from a removed block).
- Ground truth questionable: no
