# Accuracy analysis: cpp-paddlepaddle-paddleocr-childs-to-children-rename

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 6780 nodes, 6780 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 29024: renderer 0, matcher 0 |
| Painting, Full | 10 bytes wrong of 29024: renderer 10, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The field `PolyNodes Childs;` is renamed to `Children` (line 160), and the comment that mentions it
is updated: `// node index in Parent.Childs` -> `Parent.Children` (line 173).

## Painting

- **Rename inside a comment narrowed below the word (Full, 10 bytes, `None -> Update`,
  `renderer`).** In the comment on line 173, omnidiff's Full rendering paints only `s` -> `ren`. It
  leaves the shared prefix `Child` unpainted on both sides. The Full painting paints the whole word
  `Childs` / `Children`, as it does for the identifier on line 160, where omnidiff agrees. Minimal
  agrees on both lines. The runs are the same with the human mapping.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full rendering in `src/diff/text.rs`. Edits inside a comment would get a
  word-level extent, as identifier renames already do.
- Ground truth questionable: no
