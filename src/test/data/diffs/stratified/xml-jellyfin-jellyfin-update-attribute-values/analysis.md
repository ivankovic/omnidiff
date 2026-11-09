# Accuracy analysis: xml-jellyfin-jellyfin-update-attribute-values

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 156 nodes, 156 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 394: renderer 0, matcher 0 |
| Painting, Full | 88 bytes wrong of 394: renderer 88, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Jellyfin test data `emptycategory.xml`, line 2: the time-zone offsets in the `start` and `stop`
attribute values of `<programme>` change from `-0400` to `+0000`.

## Painting

There are no mapping mismatches and Minimal agrees exactly. Full, 88 bytes, all surviving with
the human mapping, so the renderer's:

- None -> Update, `00` at the end of each offset (cols 54-55 and 82-83, both sides). The Full
  painting updates the whole `-0400` -> `+0000`; omnidiff narrows the Update to the differing
  `-04` -> `+00`, as the Minimal painting does. `renderer`.
- None -> Move, the unchanged parts of the two attribute values, `"20221104130000 `,
  `" stop="20221105235959 ` and the closing `"` (same row and column on both sides). The Full
  painting accounts for every byte of the updated values and records these unchanged pieces as
  byte-identical Matches, which the label scheme shows as Move; omnidiff leaves unchanged text
  inside an updated token unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` Full narrowing of updated attribute values (whole changed
  number vs differing characters)
- Ground truth questionable: no (the Move label on in-place unchanged text comes from Full's
  account-for-every-byte convention, not from a move)
