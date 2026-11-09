# Accuracy analysis: java-defects4j-cli-36-options

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 2400 nodes, 2400 graded, N:M floor 0 visible |
| Painting, Minimal | 6 bytes wrong of 19382: renderer 6, matcher 0 |
| Painting, Full | 48 bytes wrong of 19382: renderer 48, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-36 in `Options`: `optionGroups` becomes a `LinkedHashMap<String, OptionGroup>`
(before line 61, after line 60) so groups keep insertion order, and the unused `java.util.HashMap`
import (before line 24) is removed.

## Painting

No mapping mismatches. Every painting run also appears with the human mapping, so all are the
renderer's.

- Full, 48 bytes, Move -> None: `<String, OptionGroup>();` after the renamed type (before line 61,
  after line 60). It keeps its text and its place but is pushed right by `Linked`; `FULL`'s
  `paint_displaced_moves` paints it `Move`, the Full painting does not. `renderer`.
- Minimal, 6 bytes, Update -> Insert: `Linked` of `new LinkedHashMap` (after line 60). omnidiff
  narrows `HashMap` -> `LinkedHashMap` and labels the added prefix Update; the Minimal painting
  labels it Insert. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` displaced-move painting (`paint_displaced_moves`) for type
  arguments after a renamed type; the label of a narrowed Update's added prefix
- Ground truth questionable: no
