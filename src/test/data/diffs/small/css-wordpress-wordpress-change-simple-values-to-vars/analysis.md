# Accuracy analysis: css-wordpress-wordpress-change-simple-values-to-vars

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 3418 nodes, 3418 graded, N:M floor 0 visible |
| Painting, Minimal | 6 bytes wrong of 11937: renderer 6, matcher 0 |
| Painting, Full | 8 bytes wrong of 11937: renderer 8, matcher 0 |
| Ground-truth invariant violations | none |

## The change

One declaration on line 316 of a WordPress jQuery UI stylesheet: `box-shadow: 0 0 0 2px #2271b1;`
becomes `box-shadow: 0 0 0 var(--wp-admin-border-width-focus, 1.5px) var(--wp-admin-theme-color,
#3858e9);`.
Each old value now sits as the fallback inside a new `var(...)`.

## Painting

All runs are the same with the human mapping, so they are the renderer's.

- Minimal, 6 bytes. omnidiff paints the `2px` -> `1.5px` dimension as one Update, unit included. The
  painting updates only the number (`2` -> `1.5`) and leaves `px` unpainted (before columns 20-22,
  after 57-59). omnidiff also paints the space after `2px` on the before side (inside its Update
  range, which reaches `#2271b1`). It paints Insert on the after-side space between `)` and the
  second `var(` (column 60), where the Minimal painting leaves it out. `renderer`.
- Full, 8 bytes. The same `px` and spaces, against the Full painting's Move for `px` and the space
  after it. Also the separator spaces at after columns 53 and 88, next to the two inserted
  `var(...,` texts, which the Full painting inserts and omnidiff leaves unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs: narrow the Update of a numeric value to the number when
  the unit is unchanged; and in Full, paint the separator whitespace next to inserted text.
- Ground truth questionable: no
