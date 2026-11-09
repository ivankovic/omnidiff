# Accuracy analysis: go-kubernetes-kubernetes-int-to-string

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 15 (15 visible) of 194936 nodes, 194936 graded, N:M floor 0 visible |
| Painting, Minimal | 38 bytes wrong of 537795: renderer 0, matcher 38 |
| Painting, Full | 62 bytes wrong of 537795: renderer 62, matcher 38 |
| Ground-truth invariant violations | none |

## The change

In a Kubernetes printer test, five table cells on each of three rows (lines 8368, 8413, 8477)
change from `int32(24)` to `"24"`. `description.md`: "Tricky case. The text diff is obvious, but
in the AST it violates a lot of assumptions."

## Mapping

All 15 mismatches are visible: the numbers on the three rows (`24 12 6 6 0`, `18 9 5 4 0`,
`10 5 3 2 1`). The ground truth maps each before `int_literal` to the after
`interpreted_string_literal_content` with the same text. This is a cross-kind match that keeps
what the reader sees: the digits. Omnidiff deletes the `int_literal` and inserts the string
content, through `APTED("large_flat_subtree_container")`. APTED does not relabel a node into a
different kind here even when the text is identical. The `int32(` `)` wrappers and the quotes are
delete/insert in both. `matcher`.

## Painting

The Minimal painting deletes `int32(` and `)` and inserts the quotes, and leaves the digits
alone. The Full painting does the same, and moves the digits and the separating spaces, which
shifted left on the row.

- Matcher, Minimal 38 bytes, `Delete/Insert -> None`: the 15 numbers on both sides. This is all
  of Minimal's disagreement. With the human mapping, Minimal is exact.
- Matcher, Full 38 bytes, `Delete/Insert -> Move`: the same digits. With the human mapping they
  are unpainted (`None -> Move`), so the renderer would miss them too. The two causes overlap
  here.
- Renderer, Full 24 bytes, `None -> Move`: the space after each comma between the changed cells.
  The Full painting moves them, and omnidiff leaves them unpainted with either mapping. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: the APTED relabel cost between a literal and a string content node with
  identical text (`large_flat_subtree_container` call site, `solve_large_flat_subtrees.rs`). For
  Full: Move painting of digits and separators shifted on the same row.
- Ground truth questionable: no
