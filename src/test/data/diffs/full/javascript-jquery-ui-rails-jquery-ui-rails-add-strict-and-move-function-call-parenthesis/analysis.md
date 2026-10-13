# Accuracy analysis: javascript-jquery-ui-rails-jquery-ui-rails-add-strict-and-move-function-call-parenthesis

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 3 (1 visible) of 412 nodes, 412 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

jquery-ui-rails `safe-active-element.js`: adds `"use strict";` twice (after lines 4 and 16) and
moves the closing parenthesis of the UMD wrapper, so `( function( factory ) {...} ( function( $
) {...} ) );` becomes `( function( factory ) {...} )( function( $ ) {...} );` (before lines 13
and 42, after lines 15 and 46). description.md: "A very small change results in a big change in
data flow".

## Mapping

The parenthesis move inverts two containers: before, the `parenthesized_expression` (line 3)
wraps the `call_expression`; after, the `call_expression` wraps the `parenthesized_expression`.
The ground truth pairs both containers (MatchButNotIdentical) across that inversion, and pairs the
`)` that closed the wrapper (before line 42 col 5) with the `)` that now closes it on after line
15 col 3.

- 1 visible, before line 42 col 5 `)`: omnidiff deletes it (and inserts the after `)`),
  `APTED("fast_fallback")`. The single token moved 27 lines up into a different parent, and the
  fallback's alignment does not carry a moved leaf. `matcher`.
- 2 invisible, the two inverted containers (before line 3 col 1 `parenthesized_expression`, line 3
  col 3 `call_expression`): omnidiff leaves both unmatched, reason `UnresolvedNode`. An
  ancestry-preserving mapping can keep at most one of the two pairs; omnidiff keeps neither.
  Invisible: the contents are matched. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: `APTED("fast_fallback")` / move detection for a single moved bracket
  token; container pairing after a parent-child inversion
- Ground truth questionable: no (pairing both inverted containers is not ancestry-preserving,
  but both are invisible)
