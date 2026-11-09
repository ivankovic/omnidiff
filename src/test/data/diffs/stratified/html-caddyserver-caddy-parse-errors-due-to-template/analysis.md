# Accuracy analysis: html-caddyserver-caddy-parse-errors-due-to-template

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 18634 nodes, 18614 graded, N:M floor 0 visible |
| Painting, Minimal | 25 bytes wrong of 102732: renderer 54, matcher 59 |
| Painting, Full | 25 bytes wrong of 102732: renderer 54, matcher 59 |
| Ground-truth invariant violations | none |

## The change

In a Go-template HTML file, line 24
`src="{{.Name | replace "#" "%23" | replace "?" "%3f" | html}}"` becomes
`src="{{.Name | pathEscape}}"`. description.md: "Mapping can't be solved correctly because of
parse errors".

## Painting

One painting (`Only one solution`), scored under both presets; the numbers are the same for
both.

- 25 bytes, line 24, omnidiff's mapping (`parse-error`). The `"` inside the template ends the
  before `quoted_attribute_value` early: the before value is `"{{.Name | replace "` and the rest
  (`#" "%23" | replace "?" "%3f" | html}}"`) parses as stray attribute nodes. The after value is
  the whole `"{{.Name | pathEscape}}"`. omnidiff updates the value (painting `replace "` and
  `pathEscape}}"` as Update where the painting has Delete / Insert, 9 + 13 bytes) and deletes the
  stray nodes, so the before `}}"` is painted Delete while the painting leaves `}}"` unpainted on
  both sides (3 bytes). The painting reads the text; the trees cannot line the closing `}}"` up.
- The human-mapping runs (54 bytes: the whole `replace ... html` and `pathEscape` unpainted) are
  not omnidiff's rendering: the human file leaves the 20 nodes in the broken region ungraded
  (18614 of 18634 graded), so rendering the human mapping paints nothing there. They are why
  renderer and matcher both exceed the real total.

## Verdict

- Primary cause: `parse-error`
- Other causes: `grading-artifact`
- Where a fix would go: none in the matcher; the tree-sitter HTML parse of template text inside
  an attribute value (or a text-level fallback for ERROR-adjacent leaves)
- Ground truth questionable: no
