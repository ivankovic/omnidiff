# Accuracy analysis: shellscript-mongodb-mongo-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 165 nodes, 165 graded, N:M floor 0 visible |
| Painting, Minimal | 14 bytes wrong of 814: renderer 14, matcher 0 |
| Painting, Full | 4 bytes wrong of 814: renderer 4, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The VS Code debugger install script drops the `npm install -g @vscode/vsce` step and replaces
`vsce package --skip-license` (before line 16) with `npm run package` (after line 13).

## Painting

The mapping is perfect; all runs are the same with the human mapping.

- Minimal, 14 bytes: `package` on both sides is painted Move by omnidiff (it is the first
  argument before and the second after, behind the new `run`). The Minimal painting leaves it
  unpainted. The Full painting, however, does paint `package` as Move (with its leading space),
  which is what omnidiff renders. So the two paintings differ on whether `package` moved; the
  rendering follows the Full one in both presets. `renderer` (shift by an insertion painted as a
  move, as in shellscript-ansible-ansible-a-small-add), with the paintings themselves split.
- Full, 4 bytes: whitespace attribution. The Full painting puts the space before `package` into
  the Move, deletes the space before `--skip-license` and inserts the space before `run`; the
  rendering leaves these spaces unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: move painting of a sibling shifted by an insertion in front of it
  (Minimal); separator whitespace around inserted/deleted/moved words (Full).
- Ground truth questionable: no (the Minimal and Full paintings disagree on whether `package`
  moved; both are defensible)
