# Accuracy analysis: json-kiwix-kiwix-desktop-add-a-few-change-a-few

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 20 (14 visible) of 4180 nodes, 4180 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

The Dutch locale `nl.json` (one flat object of about 190 key/value pairs) changes the value of
`"open-link-new-tab"` (line 123, `nieuwe tab` -> `nieuw tabblad`) and appends two new pairs,
`"increase-tts-speed"` and `"decrease-tts-speed"`, after `"speed"` (after lines 196-197).

## Mapping

All 20 mismatches are one wrong pairing, reason `APTED("large_flat_subtree")` (the flat root
object is diffed on its own in `solve_large_flat_subtrees`):

- The ground truth maps the `"open-link-new-tab"` pair (before line 123) to the same key on after
  line 123: key and punctuation Identical, value `string_content` Updated.
- omnidiff maps it to the newly appended `"decrease-tts-speed": "Uitspraaksnelheid verlagen."`
  (after line 197): pair and strings MatchButNotIdentical, the key `open-link-new-tab` ->
  `decrease-tts-speed` and the value as Updates, the quotes and `:` Identical. The real
  counterpart on after line 123 is then left inserted, and its new pairing shows as a 74-line
  move plus two renames. The edited pair is not byte-identical to anything; why APTED preferred a
  same-shape pair 74 lines away over the one with the identical key was not determined. `matcher`.

14 visible (the quotes, `:`, key and value text on both sides), 6 invisible (pair, string and
`string_content` containers).

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: `solve_large_flat_subtrees` / the flat APTED path for JSON objects: pair
  unmatched `pair` nodes by identical key before by shape or position.
- Ground truth questionable: no
