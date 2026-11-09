# Accuracy analysis: vimscript-neovim-neovim-only-delete

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 268 nodes, 268 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 1853: renderer 0, matcher 0 |
| Painting, Full | 1 bytes wrong of 1853: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

neovim's sudoers ftplugin drops `setlocal iskeyword+=-` (line 22) and removes `iskeyword<` from the
`b:undo_ftplugin` string (line 24).

## Painting

The run is the same with the human mapping.

- Full, 1 byte, None -> Delete: the space before `iskeyword<` on before line 24 (column 48). The
  Full painting deletes ` iskeyword<`. omnidiff's narrowed update of the string leaf deletes
  `iskeyword<` alone, the same span the Minimal painting has. Separator whitespace next to a
  narrowed deletion is not extended under Full. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full rendering in src/diff/text.rs: extend a narrowed in-string deletion
  over one adjacent separator space.
- Ground truth questionable: no
