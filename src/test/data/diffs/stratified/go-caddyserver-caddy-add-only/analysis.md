# Accuracy analysis: go-caddyserver-caddy-add-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 11108 nodes, 11108 graded, N:M floor 0 visible |
| Painting, Minimal | 5 bytes wrong of 50797: renderer 0, matcher 5 |
| Painting, Full | 7 bytes wrong of 50797: renderer 0, matcher 7 |
| Ground-truth invariant violations | none |

## The change

A `reuse_private_keys` option is added: an `if` block that sets `ap.ReusePrivateKeys` (after lines
121-125), and a new conjunct `aps[i].ReusePrivateKeys == aps[j].ReusePrivateKeys &&` (after line
595) in the long `&&` chain that compares automation policies. description.md: "There is a
preference for "&&" to be on the same line. It simply looks better."

## Mapping

- **Which `&&` is new (2 visible, `ambiguous`, `APTED("qualified_name")`).** The chain is
  left-associative. Before, the `&&` at the end of line 589 (`aps[i].OnDemand == ... &&`) joins the
  last conjunct, `RenewalWindowRatio`. After, that role is played by the `&&` at the end of the new
  line 595. Omnidiff follows the tree position: it maps before 589's `&&` to after 595's and makes
  after 594's `&&` new. The ground truth keeps each `&&` with the line it ends: before 589 -> after
  594, and the new line's `&&` is new. Both accounts are consistent mappings. The ground truth's is
  the one the description prefers, because it shows the inserted line as wholly new.

## Painting

- **Matcher-owned (Minimal 5, Full 7 bytes, `ambiguous`).** The `&&` on after line 594 is painted
  Insert, and the new line 595's ` &&` is left unpainted. In Full the before `&&` is also painted
  Move. With the human mapping there is no disagreement at all.

## Verdict

- Primary cause: `ambiguous`
- Other causes: none
- Where a fix would go: none required. If Marko wants the stated preference encoded, a tie-break for
  operator tokens in a re-associated binary chain would prefer the operator on the same line as its
  kept left operand.
- Ground truth questionable: no
