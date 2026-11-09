# Accuracy analysis: java-defects4j-codec-16-base32

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 8759 nodes, 8759 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 52145: renderer 0, matcher 0 |
| Painting, Full | 36 bytes wrong of 52145: renderer 36, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Line 99 of a decode table: the trailing value `32,` is removed and the row gains an alignment
padding and the comment `// 50-57 P-V`.

## Painting

Minimal matches exactly. Full, `None -> Insert`, line 99 after side, 36 bytes: the run of spaces
between `31,` and the new comment. The Full painting paints that padding as inserted; omnidiff's
Full rendering paints the comment but not the whitespace gap before it. Same with the human mapping.
`renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs Full gap painting: whitespace before an inserted trailing
  comment
- Ground truth questionable: no
