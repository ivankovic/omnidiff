# Accuracy analysis: yaml-gyulyvgc-sniffnet-version

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 126 nodes, 126 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 341: renderer 0, matcher 0 |
| Painting, Full | 23 bytes wrong of 341: renderer 23, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The AppImage recipe moves from Debian `bookworm` to `trixie` (line 3) and the package
`libpcap0.8` becomes `libpcap0.8t64` (line 5). The mapping is perfect and Minimal agrees byte for
byte.

## Painting

### Full, 23 bytes, None -> Update (renderer; the same with the human mapping)

Line 5: the Full painting updates the whole scalar `libpcap0.8` -> `libpcap0.8t64` (10 bytes
before, 13 after). omnidiff's Full rendering narrows the update to the appended `t64` and leaves
the common prefix `libpcap0.8` unpainted on both sides (the run covers the whole scalar because
`t64` is painted Insert, not Update). The Minimal painting records exactly omnidiff's reading
(Insert `t64`), so this is the Full style choice of painting whole renamed tokens. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` (Full): paint an updated scalar as a whole Update rather than
  narrowing it to an appended suffix.
- Ground truth questionable: no
