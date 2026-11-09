# Accuracy analysis: css-shadcn-ui-ui-add-two-bytes

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 5278 nodes, 5278 graded, N:M floor 0 visible |
| Painting, Minimal | 3 bytes wrong of 19118: renderer 3, matcher 0 |
| Painting, Full | 3 bytes wrong of 19118: renderer 3, matcher 0 |
| Ground-truth invariant violations | none |

## The change

One Tailwind `@apply` line in a stylesheet (line 153): `overscroll-none` becomes
`overscroll-y-none`. Two bytes are added inside one token.

## Painting

The fixture has one painting (`Left`), so it is scored under both presets. The same run is
present with the human mapping, so it is the renderer's.

- None -> Insert, 3 bytes, after line 153, `-y-` (columns 21-24). The painting inserts `-y` (columns
  21-23). The run is one span, and a run is extended only while the two labels differ. So omnidiff's
  label is not Insert at column 22 and not None at column 23. omnidiff therefore paints the other
  alignment, `y-` (columns 22-24), with a non-Insert label (Update or Move; the exact label is not
  printed). Both alignments of a two-character insertion between `overscroll` and `-none` are
  textually valid. Insert vs Update for a pure in-token insertion is the renderer's choice.
  `renderer` (the alignment part is `ambiguous`).

## Verdict

- Primary cause: `renderer`
- Other causes: `ambiguous`
- Where a fix would go: narrow Update rendering in src/diff/text.rs: label a pure in-token insertion
  Insert. The alignment (`-y` vs `y-`) is a tie that a rule such as "align on word boundaries" could
  break.
- Ground truth questionable: no
