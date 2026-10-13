# Accuracy analysis: go-gin-change-import

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 849 nodes, 849 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 2376: renderer 0, matcher 0 |
| Painting, Full | 1 bytes wrong of 2376: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The import alias is dropped: line 10 `validator "gopkg.in/go-playground/validator.v8"` becomes
`"gopkg.in/go-playground/validator.v8"`.

## Painting

Minimal matches exactly. Full has one byte, and it is the same with the human mapping.

- Full, `None -> Delete`, before line 10, 1 byte: the indentation tab. The Full painting has two
  Delete entries on that line: `validator` (columns 1-10), and a separate one for the tab (columns
  0-1). The after line still starts with the same tab, so nothing a reader sees deleted the
  indentation. Omnidiff leaves it unpainted. Full paints indentation that changed role, but this
  tab did not change. `ground-truth`.

## Verdict

- Primary cause: `ground-truth`
- Other causes: none
- Where a fix would go: ground truth (Marko's call)
- Ground truth questionable: yes. The Full painting deletes the leading tab of before line 10,
  and that tab is still there on after line 10.
