# Accuracy analysis: lua-neovim-neovim-if-flips-two-branches

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 68 (46 visible) of 23806 nodes, 23806 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `runtime/lua/vim/lsp/buf.lua` (lines 873-878), the commit swaps the first two branches of an
`if`. `if opts.loclist then <setloclist; lopen>` / `elseif opts.on_list then <assert;
on_list(list)>` becomes `if opts.on_list then ...` / `elseif opts.loclist then ...`. Each condition
keeps its body. The `else` branch is unchanged.

## Mapping

- **Swapped branches updated in place instead of moved (46 visible, 22 invisible), `matcher`.**
  The ground truth maps every swapped condition and statement to its identical copy in the other
  branch:
  - `opts.loclist` (line 873) -> line 876
  - `vim.fn.setloclist(0, {}, ' ', list)` (874) -> 877
  - `vim.cmd.lopen()` (875) -> 878
  - and back the other way.

  omnidiff pairs the branches by position, through the APTED run in the qualified-name pass
  (`APTED("qualified_name")`, one `.` via `LeafBetweenMatchedNeighbours`). Line 874
  `vim.fn.setloclist(...)` is matched to after line 874 `assert(vim.is_callable(...), ...)` as
  MatchButNotIdentical. Line 875 `vim.cmd.lopen()` is matched to `opts.on_list(list)`. The
  conditions are renamed (`loclist` <-> `on_list`), and leaves that do not line up are deleted and
  inserted.

  Ordered tree edit distance cannot express a swap. The `if` body sits directly under
  `if_statement`, and the `elseif` body is under `elseif_statement`, so each body changes parent.
  APTED finds that renaming in place is cheaper than delete plus insert. The swapped statements are
  byte-identical, which an identical-subtree move pass could have caught. But APTED had already
  claimed them, so no move pass ran on them. A reader sees six lines of edits instead of two
  branches that changed places.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: before `APTED("qualified_name")` (src/diff/solve_syntax_aware_matching.rs),
  pre-match unique byte-identical statements across sibling branches of the same `if` (as
  `prematch_identical_statement_siblings` does for siblings), so APTED only sees the residue
- Ground truth questionable: no
