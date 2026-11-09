# Accuracy analysis: vimscript-neovim-neovim-small-change-2

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 94 nodes, 94 graded, N:M floor 0 visible |
| Painting, Minimal | 1 bytes wrong of 678: renderer 1, matcher 0 |
| Painting, Full | 35 bytes wrong of 678: renderer 35, matcher 0 |
| Ground-truth invariant violations | none |

## The change

neovim's cabal ftplugin sets a compiler. It adds a history comment, `compiler cabal`, and
`let b:undo_ftplugin = 'compiler make'` (after lines 5 and 12-15). The old assignment becomes an
append: `let b:undo_ftplugin = 'setl com< cms<'` turns into
`let b:undo_ftplugin .= '| setl com< cms<'` (line 13 -> 18).

## Painting

All runs are the same with the human mapping. Both omnidiff and the paintings mark `=` -> `.=`
Update and the inserted lines Insert.

- Minimal, 1 byte, None -> Insert: the space after `|` on after line 18 (column 25). The Minimal
  painting inserts `| ` (columns 24-26), which ends in whitespace. Painting rule 4
  (text_painting_findings.md) says Minimal never paints whitespace at the ends of a range, and
  omnidiff trims it. `ground-truth` (minor slip).
- Full, 35 bytes, None -> Move/Insert. The Full painting splits the changed string: `'` and `setl
  com< cms<'` are Match (Move), `| ` is Insert, and the space and quote after `=` are Move as well.
  Before line 13 columns 21-38 and after line 18 columns 22-24 and 25-41 are in these runs. omnidiff
  paints only `|` inside the string, and leaves the shifted but unchanged text unpainted. That is
  the Full displacement exemption in `identical_or_move` plus narrowing of the string's Update. Here
  the Full painting follows the opposite convention: everything that moved in the line is Move.
  `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth`
- Where a fix would go: `identical_or_move` / narrow Update rendering in src/diff/text.rs, if Full
  should mark displaced text inside a changed line Move. The Minimal byte is the painting's (Marko's
  call).
- Ground truth questionable: yes, minor: the Minimal painting's Insert `| ` ends in a space, against
  painting rule 4.
