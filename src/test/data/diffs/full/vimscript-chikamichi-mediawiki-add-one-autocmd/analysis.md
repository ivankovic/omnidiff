# Accuracy analysis: vimscript-chikamichi-mediawiki-add-one-autocmd

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 26 (18 visible) of 137 nodes, 137 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

A Vim ftdetect file gets one more autocmd inside `if has("autocmd")`:
`au BufRead,BufNewFile *.mediawiki      set filetype=mediawiki` is inserted as line 3, below
the existing `*.wiki` line.

## Mapping

- 18 visible and 8 invisible, one group. The ground truth has `insert_with_children` for after
  `autocmd_statement:1` (after line 2, `*.wiki`). It maps before `autocmd_statement:1` (before line
  2, `*.wiki`) to after `autocmd_statement:2` (after line 3, `*.mediawiki`), with its `pattern` as
  Update. omnidiff maps before line 2 to after line 2, Identical in every node (reason
  `APTED("fast_fallback")`), and inserts line 3. The 13 "Insert (with children)" mismatches are the
  after line 2 nodes, which the ground truth wants inserted. The 13 Identical ones are the before
  line 2 nodes.
- Before line 2 and after line 2 are byte-identical and at the same position (`diff` reports only
  `2a3`). The ground truth's reading is that the old line was edited into the new one, and an
  identical copy of it was inserted above. That is possible if the author duplicated the line, but
  the text cannot show it, and it makes an unchanged line read as inserted. The extra spaces differ
  too: the `*.mediawiki` line has 6 spaces before `set`, the `*.wiki` line 11. So the ground truth's
  `autocmd_statement` pair is MatchButNotIdentical where omnidiff's is Identical. `ground-truth` (or
  at best `ambiguous`).

## Verdict

- Primary cause: `ground-truth`
- Other causes: `ambiguous`
- Where a fix would go: ground truth (Marko's call). omnidiff's mapping is the minimal, plain-diff
  reading.
- Ground truth questionable: yes: before line 2 is byte-identical to after line 2 and at the same
  position. The ground truth inserts after line 2 and maps before line 2 to the new `*.mediawiki`
  line 3 as an Update.
