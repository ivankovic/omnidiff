# Accuracy analysis: shellscript-genymobile-scrcpy-insert-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 172 nodes, 172 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 595: renderer 2, matcher 0 |
| Painting, Full | 2 bytes wrong of 595: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

One argument line, `    "scrcpy-linux-$VERSION.tar.gz" \`, is inserted into a multi-line
`sha256sum` command (after line 8).

## Painting

- Both presets, renderer, 2 bytes: the trailing ` \` line continuation of the inserted line. Both
  paintings include it in the Insert; the rendering paints the new string argument only. The
  continuation is not part of any node (gap text between two arguments), so the renderer has no
  inserted node to attach it to. Same with the human mapping. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: gap-text attribution in the text renderer: a line continuation between an
  inserted argument and the next line belongs to the insertion.
- Ground truth questionable: no
