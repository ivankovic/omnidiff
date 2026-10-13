# Accuracy analysis: vimscript-protesilaos-tempus-themes-vim-add-terminal-color-scheme

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 19 (15 visible) of 4187 nodes, 4187 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

The tempus_winter colour scheme gains a terminal palette (after lines 135-176). It also wraps the
`hi Normal` line and the comment above it (before lines 16-17) in
`if exists("g:tempus_enforce_background_color")`: the then branch has a copy with `ctermbg=0`, and
the
original two lines move unchanged into the `else` branch (after lines 19-20).

## Mapping

- 15 visible and 4 invisible, one group, all op Delete with reason `APTED("fast_fallback")`. The
  ground truth maps the before comment `" NOTE the ctermbg=none is for terminals with transparency`
  (line 16) and the whole `highlight_statement` on line 17 Identical into the new `else_statement`
  (after lines 19-20). omnidiff deletes the comment, the statement, and each attribute's name, `=`
  and value. Only the trailing `15` of `ctermfg=15` is not reported. The after counterparts are
  unique by text: the comment occurs once in each file, and the then branch's `hi Normal` differs
  (`ctermbg=0`). So an identical-hash move was available. Why the move passes did not take it was
  not determined. When the phase 6 fallback runs, the whole new `if_statement` is one unmatched root
  on the after side, against two separate roots on the before side. That makes an unequal gap, which
  goes through the wrap/reparent filter, then anchors, then atomic replacement. The near-copy in the
  then branch may make the wrap look ambiguous (not verified). `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: move detection for statements re-parented into a new `if`/`else`
  (`MovedSubtree`), or the wrap/reparent filter of the phase 6 fallback in
  src/diff/apted/common/residual.rs.
- Ground truth questionable: no
