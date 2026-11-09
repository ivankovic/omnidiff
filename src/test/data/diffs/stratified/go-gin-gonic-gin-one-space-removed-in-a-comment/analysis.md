# Accuracy analysis: go-gin-gonic-gin-one-space-removed-in-a-comment

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 36 nodes, 36 graded, N:M floor 0 visible |
| Painting, Minimal | 1 bytes wrong of 467: renderer 1, matcher 0 |
| Painting, Full | 1 bytes wrong of 467: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Line 1 `// Copyright 2022 Gin Core Team.  All rights reserved.` loses one of the two spaces after
`Team.`. Nothing else changes.

## Painting

There is one painting ("Only one solution"), scored under both presets. It deletes the one space
(before line 1, byte 32). Omnidiff paints nothing on the line under either preset, with its own
mapping or the human one. So a reader of omnidiff's output sees no change at all in this fixture.

- `None -> Delete`, before line 1, 1 byte, both presets: a range that is nothing but whitespace
  inside a comment gets no paint. The mapping is right (no mismatches), so this is the
  rendering. Even Full, which is meant to paint padding, does not show it. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the comment text diff and whitespace trimming in the renderer
  (`src/diff/text/render_options.rs` and the comment update path). A whitespace-only edit that
  is the entire change should still get a range, at least in Full.
- Ground truth questionable: no
