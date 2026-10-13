# Accuracy analysis: html-pandas-dev-pandas-release-banner-update

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 38 nodes, 38 graded, N:M floor 0 visible |
| Painting, Minimal | 71 bytes wrong of 311: renderer 71, matcher 0 |
| Painting, Full | 83 bytes wrong of 311: renderer 83, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A one-line announcement banner: `pandas 3.0.0 release candidate ready for testing!` becomes
`pandas 3.0 released!` and the blog link `pandas-3.0-release-candidate.html` becomes
`pandas-3.0.html`.

## Painting

One painting (`Minimal`), scored under both presets; all runs are identical with the human
mapping.

- Text and href, 65 bytes in both presets (`renderer`), Update -> Delete / Insert. The banner
  text and the `href` value are leaves mapped as Updates. omnidiff paints the narrowed spans
  (`.0 release candidate ready for testing`, ` released`, `-release-candidate`) as Update; the
  painting marks the same spans as Delete and Insert. The spans agree, the labels do not.
- `<a` and `"` of the link, 6 bytes in Minimal (`renderer`), Move -> None: the `<a>` element
  shifts left on the same line because the text before it got shorter, and omnidiff paints the
  `<a` and opening `"` tokens Move. The painting leaves them unpainted, as Minimal's
  identical-line-tail rule would.
- In Full the whole `<a href="` is painted Move on both sides (18 bytes, 12 more than Minimal;
  `grading-artifact`): Full paints same-row displacement by design, held here to the single
  Minimal painting.

## Verdict

- Primary cause: `renderer`
- Other causes: `grading-artifact`
- Where a fix would go: src/diff/text.rs: label choice for Updates the painting treats as
  replace, and Minimal Move painting of tokens displaced on the same row
- Ground truth questionable: no
