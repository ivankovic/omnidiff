# Accuracy analysis: shellscript-cockroachdb-cockroach-another-flags-in-string-example

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 173 nodes, 173 graded, N:M floor 0 visible |
| Painting, Minimal | 6 bytes wrong of 2163: renderer 6, matcher 0 |
| Painting, Full | 8 bytes wrong of 2163: renderer 8, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Inside the long string `BAZEL_SUPPORT_EXTRA_DOCKER_ARGS="-e ... -e CLOUD -e COCKROACH_DEV_LICENSE
-e TESTS ..."` (before line 16, after line 19), the flag `-e COCKROACH_DEV_LICENSE` is removed; a
three-line comment is added above.

## Painting

The mapping is perfect; all runs are the same with the human mapping.

- Both presets, 6 bytes: which `-e` goes with the deleted name. The paintings delete
  `-e COCKROACH_DEV_LICENSE ` (the `-e ` in front of the name, before columns 81-84 of line 16).
  Omnidiff's text diff of the string content deletes ` -e COCKROACH_DEV_LICENSE`, taking the ` -e`
  after `CLOUD` (columns 105-108) instead. The deleted text is the same, `-e` repeats, and a
  character diff cannot tell the two alignments apart; the painter's one keeps the flag with its
  value, which reads better, but both are correct diffs. `ambiguous` (renderer owned).
- Full only, 2 bytes: the closing `"` of the string on both sides is painted Move; the Full
  painting leaves it unpainted. It shifted because text inside the string was removed. `renderer`.

## Verdict

- Primary cause: `ambiguous`
- Other causes: `renderer`
- Where a fix would go: string-content diff in the text renderer (prefer aligning a deleted run
  so that it starts at a token boundary, `-e NAME `); no Move on a closing quote shifted by an
  edit inside the string.
- Ground truth questionable: no
