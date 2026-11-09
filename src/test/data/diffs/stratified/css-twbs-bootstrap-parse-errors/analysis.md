# Accuracy analysis: css-twbs-bootstrap-parse-errors

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 8 (7 visible) of 690 nodes, 674 graded, N:M floor 0 visible |
| Painting, Minimal | 56 bytes wrong of 2619: renderer 11, matcher 67 |
| Painting, Full | 85 bytes wrong of 2619: renderer 37, matcher 91 |
| Ground-truth invariant violations | none |

## The change

One SCSS declaration (line 18): `box-shadow: $box-shadow-lg;` becomes
`box-shadow: var(--bs-box-shadow-lg);`. The fixture is named for its parse errors. tree-sitter-css
does not know SCSS `$` variables or `@include` arguments. Both sides have an `ERROR` for
`media-breakpoint-down(lg)`. On the before side the `ERROR` swallows the nested
`.offcanvas-lg { ...` rule up to `$box-shadow-lg`, and the closing braces of lines 19-21 end up
in other parents.

## Mapping

- 7 visible and 1 invisible, lines 15-21 of before, one cause. The ground truth maps the leaves of
  the before `ERROR` across the structural difference: the `{` on line 16 to the inner `block`'s
  `{`; the `box-shadow` `identifier` to the after `property_name` (MatchButNotIdentical); `:`, `;`
  and the three `}` to their after counterparts; and the `ERROR` to the after `ERROR`. omnidiff
  deletes the `ERROR` and these leaves (reason `APTED("fast_fallback")`). It pairs the `}` on before
  line 19 with the outermost `}` on after line 21 (op Identical, same reason). The parent chains
  differ on the two sides, so no ancestor anchors these leaves. `parse-error`, with a `matcher`
  share: the fallback does not pair same-text leaves across an `ERROR` boundary.

## Painting

All wrong bytes are the matcher's. None of omnidiff's runs overlaps a run of the human-mapping
rendering.

- Minimal, 56 bytes, Delete/Insert -> None: on line 18, `box-shadow: ` and `box-shadow-lg;` are
  deleted on the before side and inserted on the after side (`;` separately). The `{` on line 16 and
  the `}` on lines 20-21 (before) and line 20 (after) are painted Delete/Insert. The painting marks
  only `$` deleted and `var(--bs-` and `)` inserted, and omnidiff agrees on those bytes.
  `parse-error` via the matcher.
- Full, 85 bytes: the same runs including the indentation, and the after `{` on line 16. The wrongly
  paired `}` is painted Move on before line 19 and after line 21. On `box-shadow-lg` the Full
  painting has an Update (Match before columns 19-32 to after columns 22-40), where omnidiff paints
  Delete/Insert. `parse-error` via the matcher.
- With the human mapping the renderer would miss `$`, `var(--bs-` and `)` (11 bytes Minimal, 37
  Full), but omnidiff's own rendering gets those right. Those value nodes have no ground-truth
  entry.

## Verdict

- Primary cause: `parse-error`
- Other causes: `matcher`
- Where a fix would go: phase 6 fallback (`resolve_residual_forest_via_myers_lcs`,
  src/diff/apted/common/residual.rs), so that it pairs equal leaves under an `ERROR` with their
  structured counterparts. The root cause is in the grammar (SCSS in tree-sitter-css).
- Ground truth questionable: no
