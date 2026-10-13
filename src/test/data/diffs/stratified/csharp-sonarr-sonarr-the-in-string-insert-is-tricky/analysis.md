# Accuracy analysis: csharp-sonarr-sonarr-the-in-string-insert-is-tricky

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 21086 nodes, 21086 graded, N:M floor 0 visible |
| Painting, Minimal | 15 bytes wrong of 156009: renderer 15, matcher 0 |
| Painting, Full | 6 bytes wrong of 156009: renderer 6, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Two edits in a 21k-node parser file: inside the verbatim regex of `NormalizeRegex` (line 520) the
article alternative `a` becomes the character class `[aà]`, and on line 848 `.ToLower()` becomes
`.ToLowerInvariant()`.

## Painting

Mapping is perfect and every run is identical with the human mapping, so all of it is renderer-owned
(columns below are byte columns; `à` is two bytes).

- Line 520, both presets, 6 bytes: the paintings record `a` -> `[aà]` as three insertions (`[`,
  `à`, `]`) around an untouched `a` (Minimal leaves `a` unpainted; Full matches it as a Move,
  shifted one column). omnidiff's text diff inside the string token paints a replacement instead:
  before `a` as Update (1 byte) and after `[aà]` as one Update (5 bytes). The pure-insertion reading
  needs the inner diff to keep the single shared character `a` as an anchor, which omnidiff's
  narrowing does not. `renderer`.
- Line 848, Minimal only, 9 bytes, Update -> Insert: `Invariant` appended to `ToLower`. omnidiff
  paints the added part of the updated identifier with the Update label; the Minimal painting
  labels it Insert. The Full painting of the same fixture records this identifier as a Match
  (Update) and agrees with omnidiff, so the two paintings themselves label it differently.
  `ambiguous`.

## Verdict

- Primary cause: `renderer`
- Other causes: `ambiguous`
- Where a fix would go: the text diff inside a string token (src/diff/text.rs), which does not keep
  the one shared character `a` as an anchor; the Update/Insert label on line 848 is a convention the
  two paintings do not agree on
- Ground truth questionable: no (the Minimal and Full paintings label `Invariant` differently,
  Insert vs Update, which is a convention question rather than an error)
