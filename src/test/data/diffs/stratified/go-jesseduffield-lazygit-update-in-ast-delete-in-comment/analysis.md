# Accuracy analysis: go-jesseduffield-lazygit-update-in-ast-delete-in-comment

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1330 nodes, 1330 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 5604: renderer 0, matcher 0 |
| Painting, Full | 36 bytes wrong of 5604: renderer 36, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In vendored testify, the type `CompareType` is renamed `compareResult` at five uses (lines 9, 53,
62, 71, 80). gofmt rewrites the doc-comment examples from `//    assert.Is...(...)` (four spaces)
to `//<tab>assert.Is...(...)` on 12 lines (49-51, 58-60, 67-69, 76-78).

## Painting

Minimal matches exactly: the Minimal painting updates the five type names and leaves the comments
alone. In Full, all 36 bytes are the same with the human mapping, so they are the renderer's.

- Full, `None -> Delete`, 3 bytes on each of the 12 comment lines (before side). The Full painting
  deletes three of the four spaces after `//`. The fourth space is left unpainted against the
  tab. Omnidiff's Full rendering paints nothing in these comments, because a whitespace-only edit
  inside a comment gets no range. This is the same behaviour as in
  `go-gin-gonic-gin-whitespace-in-comment`. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the comment text diff and whitespace handling in the renderer. Under
  Full, a whitespace-only edit inside a comment should be painted.
- Ground truth questionable: no
