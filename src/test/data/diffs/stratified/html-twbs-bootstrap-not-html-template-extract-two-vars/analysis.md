# Accuracy analysis: html-twbs-bootstrap-not-html-template-extract-two-vars

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 115 nodes, 110 graded, N:M floor 0 visible |
| Painting, Minimal | 151 bytes wrong of 659: renderer 182, matcher 139 |
| Painting, Full | 159 bytes wrong of 659: renderer 198, matcher 139 |
| Ground-truth invariant violations | none |

## The change

A Hugo heading render hook extracts two variables: new first lines
`{{- $id := .Anchor | safeURL -}}` and `{{- $text := .Text | safeHTML -}}`, and the four uses of
`.Anchor | safeURL` / `.Text | safeHTML` (lines 1 and 3 before) become `$id` / `$text`.
description.md: a template, not fully parsed, and it needs N:M even if it parsed.

## Mapping

- The two `id`/`href` attribute values (2 visible; `ambiguous`, `StructurallyIdenticalAncestor`).
  The ground truth inserts the after values `{{ $id }}` (after line 3 col 20) and `#{{ $id }}`
  (after line 5 col 30) and leaves the before values ungraded (their expressions moved, N:M, into
  the new `$id :=` line, which is a text node). omnidiff maps each before value to the after value
  in the same attribute as an Update. Both readings are defensible: the attribute keeps its role,
  but its content was extracted elsewhere.

## Painting

Both paintings move the two before `.Anchor | safeURL` and the two `.Text | safeHTML` to the
single occurrence in each new definition line (2:1 Match entries), and insert `$id` / `$text`.

- N:M extraction (Minimal 103 bytes, Full 99; `nm-floor`): before lines 1 and 3, the four
  expressions are Update -> Move (66 bytes); after lines 1-2 the definitions' expressions are
  Insert -> Move (37 / 33 bytes). omnidiff maps the expressions to the `$id` / `$text` uses and
  inserts the definition lines; with the human mapping they render unpainted instead, also wrong.
  No one-to-one mapping, and no tree mapping into a text node, can show a 2:1 move.
- `$id` / `$text` uses (16 bytes, both presets; `ambiguous`): Update -> Insert. They are painted
  as part of the updated values (and, for `{{ $text }}` on line 3, of the text node the ground
  truth itself maps as an Update).
- Displaced punctuation (Minimal 32 bytes, Full 44; `renderer`, also with the human mapping):
  `">`, `" aria-label="` and in Full `"></a>` shift left on the same row because the values got
  shorter; omnidiff paints them Move, the paintings leave them unpainted.

## Verdict

- Primary cause: `nm-floor`
- Other causes: `ambiguous`, `renderer`
- Where a fix would go: none for the extraction (2:1 move into a text node); Move painting of
  same-row displaced punctuation in src/diff/text.rs
- Ground truth questionable: no
