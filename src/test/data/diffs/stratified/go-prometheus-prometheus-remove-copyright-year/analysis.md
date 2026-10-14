# Accuracy analysis: go-prometheus-prometheus-remove-copyright-year

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 62 nodes, 62 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 2011: renderer 0, matcher 0 |
| Painting, Full | 1 bytes wrong of 2011: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The copyright header drops the year: `// Copyright 2020 The Prometheus Authors` becomes
`// Copyright The Prometheus Authors`.

## Painting

Minimal matches exactly (both delete `2020`). Full has one byte, and it is the same with the human
mapping.

- Full, `None -> Delete`, before line 1, 1 byte: the space before `2020`. The Full painting
  deletes ` 2020` with its separating space. Omnidiff's comment diff deletes only the word under
  both presets. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the intra-comment word diff under Full. It should take the separating
  space of a deleted word.
- Ground truth questionable: no
