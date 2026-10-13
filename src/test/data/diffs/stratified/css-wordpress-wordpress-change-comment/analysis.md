# Accuracy analysis: css-wordpress-wordpress-change-comment

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 11650 nodes, 11650 graded, N:M floor 0 visible |
| Painting, Minimal | 19 bytes wrong of 41043: renderer 19, matcher 0 |
| Painting, Full | 19 bytes wrong of 41043: renderer 19, matcher 0 |
| Ground-truth invariant violations | none |

## The change

One SCSS comment heading (line 663): `/** === Verse === */` becomes
`/** === Poetry (Verse) === */`.

## Painting

Both presets give the same runs, and so does the human mapping, so they are the renderer's. 19 bytes
in each preset.

- Before line 663, `Verse` (5 bytes): omnidiff paints Update. Minimal leaves it unpainted; Full
  paints it Move (Match to after columns 16-21).
- After line 663, `Poetry (Verse)` (14 bytes): omnidiff paints the whole span Update. The paintings
  Insert `Poetry (` (columns 8-16) and `)` (21-22), and leave `Verse` unpainted (Minimal) or Move
  (Full).

omnidiff narrows the comment's Update to the part between the common prefix `/** === ` and the
common suffix ` === */`. It does not see that the old middle `Verse` survives inside the new one, so
a pure insertion around a kept word is painted as a replacement. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: narrow Update rendering of a leaf in src/diff/text.rs: a character or word
  diff inside the changed middle, so that an insertion around kept text is painted Insert.
- Ground truth questionable: no
