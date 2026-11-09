# Accuracy analysis: python-api-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 434 nodes, 434 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 1528: renderer 4, matcher 0 |
| Painting, Full | 154 bytes wrong of 1528: renderer 154, matcher 0 |
| Ground-truth invariant violations | none |

## The change

From description.md: an API client gains type hints (`user_id: int`, `-> Optional[...]`), a
`timeout` parameter, a v1 -> v2 URL, `try`/`except` with `raise_for_status()`, and a new function
`get_user_info`. The `from typing import ...` line inserted at line 2 shifts every later line by one.

## Painting

Both paintings exist (`Minimal`, `Full`). Every run is identical with the human mapping, so
nothing is the matcher's.

Minimal (4 bytes):

- **None -> Move, 2 bytes: before line 6, after line 9, the `)` of `requests.get(url)`.** The
  painting moves `response = requests.get(url` and, separately, the `)` that `, timeout=timeout`
  pushed right. omnidiff paints the first range Move but drops the lone `)` range.
  `structural_punctuation` is off in Minimal, and `)` is in `STRUCTURAL_PUNCTUATION`. So in
  omnidiff's rendering the `(` is painted and its `)` is not, which breaks the bracket-fate rule
  (text_painting_findings.md rule 2). The painting keeps the pair together. `renderer`.
- **None -> Insert, 2 bytes: after line 5 col 31 and after line 17 col 30.** The painting's Inserts
  for ` with improved error handling` and ` with type hints` in the docstrings begin with the space.
  Minimal trims leading whitespace from every range, and rule 4 says whitespace at range ends is
  never painted. Here the painting differs from its own convention. `ground-truth`.

Full (154 bytes):

- **None -> Move, 32 bytes: `(user_id` on both `def` lines (before lines 3, 13; after lines 4,
  16).** The painting moves `(`, `user_id` and `)`. The tree mapping re-parents `user_id` into a new
  `typed_parameter`. omnidiff paints `)` Move, which was pushed right by `: int, ...`. It leaves `(`
  and `user_id` unpainted: they kept their column, and a row-only shift is read as "an unrelated edit
  above". omnidiff's Full rendering therefore splits the `(`/`)` pair. `renderer`.
- **Move -> None, 16 bytes: the trailing `:` of both `def` lines and the closing `"""` of both
  docstrings, on both sides.** Text inserted before them pushed them right. The painting treats them
  as an unchanged line tail (rule 3). Under Full, the `shifted_by_an_edit_beside_it` exemption in
  `identical_or_move` needs `s.start_row == d.start_row`, and the inserted import line breaks that,
  so the column shift paints Move. Minimal accepts the row shift and agrees with the painting.
  `renderer`.
- **None -> Move, 88 bytes: the unchanged pieces `"https://api.example.com/v` and
  `/users/{user_id}"` of the updated f-string URL (before line 5, after line 6).** Both trees map the
  `string_content` as an Update. omnidiff narrows the Update to `1` -> `2` and leaves the rest
  unpainted. The Full painting marks the rest Move. `RenderOptions` documents the narrow reading of
  strings as deliberate in both presets. The 88 bytes include 2 bytes of spaces before the new
  `# API version updated` comment (None -> Insert). `renderer`.
- **Indentation, 16 bytes.** The old 4-space indent of the `response = ...` line (before line 6) is
  matched by the painting to the inner 4 of the new 8-space indent, and the outer 4 are Insert
  (after line 9). omnidiff paints neither. After line 7 is a whitespace-only line (`    `) that
  omnidiff paints Insert and the painting does not. `renderer`.
- **None -> Insert, 2 bytes: the space between `)` and the new `->` (after lines 4, 16).** It is gap
  text that omnidiff does not paint. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth` (2 bytes: leading spaces in two Minimal Inserts)
- Where a fix would go: `src/diff/text.rs` `identical_or_move`: the FULL row-index gating of
  `shifted_by_an_edit_beside_it`, and same-column re-parented nodes not painted; the Minimal
  structural-punctuation filter dropping a bracket whose partner is painted
- Ground truth questionable: yes: the Minimal painting's two docstring Inserts start with a space,
  against rule 4 of text_painting_findings.md (2 bytes)
