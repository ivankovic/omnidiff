# Accuracy analysis: tsx-mui-material-ui-move-import

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 390 nodes, 390 graded, N:M floor 0 visible |
| Painting, Minimal | 94 bytes wrong of 1971: renderer 94, matcher 0 |
| Painting, Full | 94 bytes wrong of 1971: renderer 94, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`import SectionHeadline from 'docs/src/components/typography/SectionHeadline';` (before line 5)
moves up to after line 3 and its path changes to `'@mui/docs/SectionHeadline'`.

## Painting

One painting (`Only one solution`), scored under both presets; all runs are identical with the
human mapping, so they are the renderer's.

- 94 bytes, None -> Update, before line 5 and after line 3 (`renderer`). The painting matches the
  whole import statement as one Update entry. omnidiff paints only the differing part of the path
  string and leaves `import SectionHeadline from '` (29 bytes per side) and `/SectionHeadline';`
  (18 per side) unpainted, although the statement moved two lines; the narrowed update hides the
  move.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs, narrowing of an Update inside a statement that also
  moved (the unchanged parts of a moved and updated import get no Move or Update paint)
- Ground truth questionable: no
