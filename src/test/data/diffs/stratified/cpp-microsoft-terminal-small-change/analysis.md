# Accuracy analysis: cpp-microsoft-terminal-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 5814 nodes, 5814 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 35021: renderer 0, matcher 0 |
| Painting, Full | 37 bytes wrong of 35021: renderer 37, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`const bool hasColorSpaceId = options.at(1).has_value();` becomes
`const bool validColorSpace = options.at(1).value_or(0) == 0;` (line 131). Its use on line 139
changes from `!hasColorSpaceId` to `validColorSpace`, and one comment word changes (`empty` ->
`zero`, line 128).

## Painting

- **Unchanged parts of the updated line (Full, 37 bytes, `None -> Move`, `renderer`).** The Full
  painting matches ` = options.at(1).` and the `)` of the call on line 131 as byte-identical spans,
  so they read as Move. Omnidiff leaves unchanged text that stays in place unpainted. The rename and
  the `has_value()` -> `value_or(0) == 0` change are painted the same way by both. The runs are the
  same with the human mapping. Minimal agrees completely.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full rendering in `src/diff/text.rs`, for the unchanged in-place parts of an
  updated statement. The Full painting's choice to mark them Move is unusual. Other Full paintings
  leave identical line tails unpainted (rule 3).
- Ground truth questionable: no
