# Accuracy analysis: cpp-libreoffice-add-const

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 12024 nodes, 12024 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 51068: renderer 0, matcher 0 |
| Painting, Full | 1 byte wrong of 51068: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

One parameter gains a qualifier: `operator= (ResourceHolder & rhs)` becomes
`operator= (const ResourceHolder & rhs)` (line 831).

## Painting

- **Space after `const` (Full, 1 byte, `None -> Insert`, `renderer`).** The Full painting paints the
  inserted `const` together with the space that follows it. Omnidiff's Full rendering paints only
  `const`. The run is the same with the human mapping.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full rendering in `src/diff/text.rs`. It would include the separating space
  after an inserted leading token.
- Ground truth questionable: no
