# Accuracy analysis: typescript-microsoft-typescript-add-target-3

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 49 nodes, 49 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 252: renderer 0, matcher 0 |
| Painting, Full | 22 bytes wrong of 252: renderer 22, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A TypeScript compiler test file gets a `// @target: es2015` directive comment as its new first
line, in front of `var x = 10;`. The file starts with a UTF-8 byte-order mark, which stays at the
start of line 1 (now the comment). The mapping is perfect; the single painting ("Only one
solution") is scored under both presets, and Minimal agrees exactly.

## Painting

### Full, 22 bytes, Move -> None (renderer; the same with the human mapping)

`var x = 10;` (before line 1, after line 2, 11 bytes each side). omnidiff's Full rendering paints
it Move: the statement went down one row because of the inserted line and also changed byte
column, from 3 (after the 3-byte BOM) to 0. The BOM is invisible, so to a reader the statement
just moved down below an inserted comment, which Full does not paint as Move elsewhere. The
painting leaves it unpainted. Wrong on Full's own terms, so `renderer` rather than a preset
mismatch.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` (Full) displacement check: ignore a leading BOM when
  comparing columns (or compare display columns).
- Ground truth questionable: no
