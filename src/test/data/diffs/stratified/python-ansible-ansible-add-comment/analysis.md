# Accuracy analysis: python-ansible-ansible-add-comment

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 131 nodes, 131 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 1824: renderer 0, matcher 0 |
| Painting, Full | 2 bytes wrong of 1824: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In an ansible-test sanity fixture (`.../plugin_utils/check_pylint.py`), the commit appends a
trailing comment to line 17: `foo = {}.keys()` becomes
`foo = {}.keys()  # should trigger disallowed-name, but doesn't in pylint 4.0.0, ...`.

## Painting

- **None -> Insert, after line 17, 2 bytes (the two spaces before `#`), Full, `renderer`.** The Full
  painting paints the gap between `keys()` and the new comment as part of the insertion. omnidiff
  paints the comment Insert but leaves the gap unpainted. That gap is text between two nodes that
  belongs to neither. The run is the same with the human mapping. Minimal leaves the gap out, as its
  convention does, and agrees.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full gap-text painting in src/diff/text.rs: attribute whitespace between an
  unchanged node and an inserted trailing node to the insertion
- Ground truth questionable: no
