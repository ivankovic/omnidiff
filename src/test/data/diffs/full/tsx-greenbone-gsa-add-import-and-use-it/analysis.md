# Accuracy analysis: tsx-greenbone-gsa-add-import-and-use-it

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 6002 nodes, 6002 graded, N:M floor 0 visible |
| Painting, Minimal | 80 bytes wrong of 18410: renderer 42, matcher 38 |
| Painting, Full | 139 bytes wrong of 18410: renderer 101, matcher 38 |
| Ground-truth invariant violations | none |

## The change

A scanner-component test imports `GREENBONE_SENSOR_SCANNER_TYPE` next to `OPENVASD_SCANNER_TYPE`,
which makes Prettier rewrap the named-import list over four lines (before line 9 / after lines
9-12), and uses it for a scanner's `type`, replacing `undefined` (line 264 / after 267).

## Mapping

- Before line 264 `undefined`, 1 visible: the ground truth updates `undefined` to the identifier
  `GREENBONE_SENSOR_SCANNER_TYPE`. omnidiff deletes it and inserts the identifier, reason
  `APTED("greedy_anchor_block")` (src/diff/solve_greedy_anchor_blocks.rs). TSX parses `undefined`
  as its own node kind `undefined`, not an identifier, and `UnitCostModel::ren` prices a cross-kind
  pair above delete + insert unless `update_allowed_from_masks` allows it (identifier-like on both
  sides, or a shared operator family; `undefined` is in neither, src/diff/nodes.rs). So a value
  replacement in the same slot is shown as delete + insert. `matcher`.

## Painting

Matcher-owned, 38 bytes in each preset (Delete/Insert -> Update): `undefined` (9) and
`GREENBONE_SENSOR_SCANNER_TYPE` (29) on line 264 / after 267, following the mapping above.
`matcher`.

Renderer-owned (identical with the human mapping):

- Minimal, 42 bytes, Move -> None: `OPENVASD_SCANNER_TYPE` on before line 9 / after line 11. The
  rewrap puts it on its own line, so it changes row and column and omnidiff paints it Move; the
  painting treats the rewrapped, otherwise unchanged import as unchanged and paints only the
  inserted name. `renderer`.
- Full, 101 bytes, Move -> None: the same identifier with its `,` (22), plus the rewrapped tail
  `} from 'gmp/models/scanner';` (49 on the before side, 28 on after line 12), and the `,` after the
  updated `type:` value on line 264/267 (1 + 1), which slides right on its row while the three
  added import lines change its row index, so Full's same-row exemption does not apply. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`
- Where a fix would go: `identical_or_move` in src/diff/text.rs for unchanged pieces of a list
  rewrapped onto new lines; and `update_allowed_from_masks` (src/diff/nodes.rs), which does not let
  TSX `undefined` update into an identifier
- Ground truth questionable: no
