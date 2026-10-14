# Accuracy analysis: html-gohugoio-hugo-template-not-pure-html

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 68 nodes, 65 graded, N:M floor 0 visible |
| Painting, Minimal | 36 bytes wrong of 381: renderer 0, matcher 36 |
| Painting, Full | 88 bytes wrong of 381: renderer 48, matcher 40 |
| Ground-truth invariant violations | none |

## The change

A Hugo template wraps the `<h2>...</h2>` and `{{ .Summary }}` of a range loop in a new
`<section>` (after lines 4-7). description.md: the template syntax breaks HTML parsing, so the
AST needs N:M, which it would not if it parsed correctly.

## Painting

The template text parses as plain `text` nodes. Before has one trailing text
`{{ .Summary }} {{ end }} {{ end }}` (lines 5-7); after splits it into `{{ .Summary }}` inside the
section and `{{ end }} {{ end }}` after it. The ground truth leaves these three text nodes
ungraded (65 of 68 graded), so the mapping score is 0 mismatches and the disagreement shows
only in the painting.

- Matcher, `{{ end }}` lines (Minimal 36 bytes, Full 40; `parse-error`): omnidiff maps the
  before trailing text to the after `{{ .Summary }}` text, so the two `{{ end }}` lines are
  painted Update on the before side (lines 6-7) and the after `{{ end }}` text is painted Insert
  (lines 8-9). The painting leaves them unpainted. A one-to-one mapping of the split text node
  cannot keep both halves unchanged.
- Full, `{{ .Summary }}` text (28 bytes; `parse-error`, also with the human mapping): the Full
  painting moves it into the section (before line 5) and inserts its new indentation; omnidiff
  leaves it unpainted because it sits inside the same text node as above.
- Full, indentation (20 bytes; `renderer`, also with the human mapping): the 4-space indentation
  of the moved `<h2>` and `{{ .Summary }}` lines is Move in the Full painting and the extra
  2 spaces per after line are Insert; omnidiff leaves re-indentation unpainted.

## Verdict

- Primary cause: `parse-error`
- Other causes: `renderer`
- Where a fix would go: none in the matcher (template text parsed as HTML text; one-to-one
  cannot follow the split); Full re-indentation painting in src/diff/text.rs
- Ground truth questionable: no
