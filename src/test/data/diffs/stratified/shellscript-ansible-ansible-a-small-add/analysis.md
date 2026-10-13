# Accuracy analysis: shellscript-ansible-ansible-a-small-add

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 83 nodes, 83 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 640: renderer 4, matcher 0 |
| Painting, Full | 114 bytes wrong of 640: renderer 114, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A test script gets `set -eux -o pipefail`, and `diff -w <(...) success.json` (before line 4)
gets a new first argument: `diff --unified -w <(...) success.json` (after line 6).

## Painting

One painting (`Only one solution`), scored under both presets. The mapping is perfect and the
runs are the same with the human mapping: all renderer.

- Minimal, 4 bytes: `-w` on both sides is painted Move; the painting leaves it unpainted. It is
  shifted one argument position by the inserted `--unified`, not moved.
- Full, 114 bytes: the same, extended over all following arguments,
  `-w <(ansible-inventory -i ./test.yml --list) success.json`, on both sides.
- Same pattern as kotlin-nextcloud-android-small-change: a sibling shifted by an insertion in
  front of it is rendered as a move. The Full form is wrong on its own terms (nothing in the tail
  changed), so `renderer`, not a grading artifact.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: move painting in the text renderer: siblings whose order relative to
  their mapped neighbours is unchanged should not be painted as moved when a new sibling is
  inserted in front.
- Ground truth questionable: no
