# Accuracy analysis: vimscript-neovim-neovim-add-two-functions-and-modify-a-few-lines

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 18 (14 visible) of 11647 nodes, 11647 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

A matchit.vim update. It adds a custom-match-function hook (after lines 72-91) and a new helper
`s:Append` (after lines 365-376). It rewrites two identical lines (before 94 and 625):
`let match_words = match_words .. (strlen(match_words) ? "," : "") .. default` becomes
`let match_words = s:Append(match_words, default)`. It also changes the un-escape substitutions and
the date in the header.

## Mapping

- 14 visible and 4 invisible, 7 + 2 for each of the two rewritten lines, one cause. The ground truth
  deletes the `strlen(match_words)` call, inserts the `s:Append(...)` call, keeps `match_words`
  mapped, and maps the trailing `default` (before column 74) to the call's second argument. omnidiff
  pairs the two `call_expression`s (MatchButNotIdentical): it updates `strlen` into `Append` and
  maps the `(` and `)` Identical, all with reason `APTED("fast_fallback")`. Then it deletes the
  before `default` and inserts the after one. With the calls paired, the after `default` lies inside
  the new call while the before one lies outside the old call, so tree edit distance cannot keep it.
  Its cost model prefers a one-leaf rename plus two kept brackets to the ground truth's delete and
  insert of whole calls. On line 625 the `default` pair is a 1:1 group in the ground truth, reported
  as "matched 0". `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the APTED cost model behind the phase 6 fallback. Pairing calls to
  differently named functions (`strlen` vs `s:Append`) should cost more than giving up an identical
  identifier.
- Ground truth questionable: no
