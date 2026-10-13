# Accuracy analysis: go-fatedier-frp-build-comment-insert-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 322 nodes, 322 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 1063: renderer 2, matcher 0 |
| Painting, Full | 2 bytes wrong of 1063: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A vendored build constraint comment gains a platform: line 5 `// +build !linux,!netbsd` becomes
`// +build !aix,!linux,!netbsd`.

## Painting

There is one painting ("Only one solution"), scored under both presets. The same 2 bytes are wrong
in each, and they are the same with the human mapping, so they are the renderer's.

- After line 5: the painting inserts `!aix,` (bytes 170..175). Omnidiff's character diff inside
  the comment puts the insert at `aix,!` (171..176). The text is the same length and either
  alignment is a valid edit script, but the painting's alignment follows the tokens a reader sees.
  This counts 1 byte `None -> Insert` and 1 byte `Insert -> None`. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the intra-comment character diff in the renderer. Slide an ambiguous
  insert so that it starts at a token boundary.
- Ground truth questionable: no
