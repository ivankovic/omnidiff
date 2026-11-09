# Accuracy analysis: html-gohugoio-hugo-update-href-template

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 392 nodes, 392 graded, N:M floor 0 visible |
| Painting, Minimal | 140 bytes wrong of 1345: renderer 140, matcher 0 |
| Painting, Full | 150 bytes wrong of 1345: renderer 150, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Line 17's `href="{{.Site.Params.ghrepo}}blob/master/content/{{.Lang }}/{{.File.Path}}"` becomes
`href="{{ .Site.Params.ghrepo }}blob/master/content/{{ .Language.Lang }}/{{ .File.Path }}"`:
spaces inside the template braces and `.Language` added. A missing final newline is also added.

## Painting

All runs are identical with the human mapping, so they are the renderer's.

- Minimal, 140 bytes (64 before, 76 after, line 17; `renderer`), Update -> None. The attribute
  value is one leaf mapped as an Update. omnidiff trims only the common `{{` prefix and `}}`
  suffix and paints everything between as Update. The Minimal painting inserts only `.Language`
  and leaves the whitespace-only changes unpainted.
- Full, 150 bytes (68 + 82; `renderer`), None -> Move/Insert. The Full painting splits the value
  into Matched pieces (graded Move because columns shift) and Inserts for each new space and
  `.Language`; omnidiff's single narrowed Update matches none of them.

Both are the same limitation: no token-level diff inside an updated string leaf.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs, Update narrowing of attribute-value/string leaves
  (token or character diff instead of common prefix/suffix)
- Ground truth questionable: no
