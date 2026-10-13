# Accuracy analysis: shellscript-ansible-ansible-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 219 nodes, 219 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 1260: renderer 2, matcher 0 |
| Painting, Full | 0 bytes wrong of 1260: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`cd ../connection` (line 7) becomes `pushd ../connection`, and a `popd` plus one more
`ansible-playbook` run are added at the end.

## Painting

One painting (`Only one solution`), scored under both presets; Full agrees exactly.

- Minimal, renderer, 2 bytes: the painting paints the whole word `cd` -> `pushd` as Update. The
  Minimal rendering narrows the update to the differing characters and leaves the shared final
  `d` unpainted on both sides (before column 1, after column 4). Same with the human mapping.
  Narrowing an update of a short command name below the word is a rendering choice the painter
  does not make. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Minimal Update narrowing in the text renderer: do not narrow inside a short
  identifier/word (common suffix of one character).
- Ground truth questionable: no
