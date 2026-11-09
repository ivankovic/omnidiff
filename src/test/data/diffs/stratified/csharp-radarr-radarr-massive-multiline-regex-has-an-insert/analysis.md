# Accuracy analysis: csharp-radarr-radarr-massive-multiline-regex-has-an-insert

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 9154 nodes, 9154 graded, N:M floor 0 visible |
| Painting, Minimal | 8 bytes wrong of 43613: renderer 8, matcher 0 |
| Painting, Full | 158 bytes wrong of 43613: renderer 158, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The commit adds Georgian language detection: one alternative
`(?<georgian>\b(?:georgian|geo|ka|kat)\b)|` inserted as a new line inside a large multi-line
verbatim regex string (after line 43), and a matching `if (match.Groups["georgian"].Success) {
languages.Add(Language.Georgian); }` block (after lines 442-445).

## Painting

Mapping is perfect and every run is identical with the human mapping, so all of it is
renderer-owned.

- Insertion inside the string slid by one repeated prefix, both presets: the regex is one string
  token, so the inserted line is found by the text diff inside it. Every line of the regex starts
  with the same 76 spaces and `(?<`, so the inserted text can be placed either as
  `(?<georgian>…|` + newline + indentation (line-aligned, what the painting paints) or as
  `georgian>…|` + newline + indentation + `(?<` (shifted right by the shared prefix). omnidiff's
  rendering takes the shifted form: it misses `(?<` at the start of after line 43 (None -> Insert)
  and paints `(?<` at the start of after line 44 (Insert -> None). Minimal 3 + 3 bytes; Full, where
  the indentation counts too, 79 + 79 bytes. Both alignments describe the same edit, but the
  line-aligned one is the one a reader sees. `renderer`.
- Minimal only, None -> Insert, after lines 443 and 445: the lone `{` and `}` of the inserted if
  block, dropped by the Minimal structural-punctuation filter (`is_structural_only`,
  src/diff/text/render_options.rs); the painting includes them. 2 bytes. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the text diff inside a multi-line string token (src/diff/text.rs): prefer
  the line-aligned placement of an insertion that can slide over a repeated prefix; and the Minimal
  structural-punctuation filter for the braces
- Ground truth questionable: no
