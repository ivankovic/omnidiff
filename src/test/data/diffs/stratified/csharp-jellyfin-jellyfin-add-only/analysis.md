# Accuracy analysis: csharp-jellyfin-jellyfin-add-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 10147 nodes, 10147 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 46255: renderer 0, matcher 0 |
| Painting, Full | 44 bytes wrong of 46255: renderer 44, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In Jellyfin's `BackupService.cs`, `typeof(HistoryRow)` becomes `typeof(IQueryable<HistoryRow>)`
(line 300), and a new line `var schemaName = dbContext.Model.FindEntityType(...)...;` is inserted
(after line 311).

## Painting

Only a Minimal painting exists, so both presets are scored against it. Minimal matches exactly.
The Full runs are the same with the human mapping.

- Full, `Insert -> None`, 24 bytes: the indentation of the inserted `var schemaName` line (after
  line 311). Full paints indentation by design, and the Minimal painting does not.
  `grading-artifact`.
- Full, `Move -> None`, 20 bytes: `HistoryRow` on line 300, both sides. It is now the type argument
  of the inserted `IQueryable<...>`, so it moved under a new parent and shifted right, and Full
  paints it grey Move. This is not only a Minimal/Full difference: where a Full painting exists
  for the same situation (`go-cockroachdb-cockroach-add-only`, a block wrapped by a new if), it
  leaves the wrapped node unpainted. `renderer`.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: `renderer`
- Where a fix would go: Full Move painting of a node that was only wrapped by an inserted parent
  (`src/diff/text.rs`, `identical_or_move`). The indentation bytes go away only with a Full
  painting.
- Ground truth questionable: no
