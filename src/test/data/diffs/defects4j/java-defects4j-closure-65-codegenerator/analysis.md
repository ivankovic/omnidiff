# Accuracy analysis: java-defects4j-closure-65-codegenerator

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 17818 nodes, 17818 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 75306: renderer 0, matcher 0 |
| Painting, Full | 8 bytes wrong of 75306: renderer 8, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Line 1015: the escape emitted for a NUL character changes from `"\\0"` to `"\\000"`.

## Painting

Minimal matches exactly (Insert of `00`). Full, `None -> Update`, line 1015, 3 bytes before (`\\0`)
and 5 after (`\\000`): the Full painting marks the string's whole content, escape included, as one
Update, while omnidiff's Full rendering narrows it to the inserted `00` as Minimal does. Same with
the human mapping. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs Full rendering of a changed string content (affix narrowing
  across an escape)
- Ground truth questionable: no
