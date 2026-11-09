# Accuracy analysis: rust-add-comments-and-real-new-logic

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 85 (54 visible) of 7827 nodes, 7827 graded, N:M floor 0 visible |
| Painting, Minimal | 230 bytes wrong of 34087: renderer 4, matcher 232 |
| Painting, Full | 270 bytes wrong of 34087: renderer 12, matcher 278 |
| Ground-truth invariant violations | none |

## The change

Handmade. Adds `status` and `rejection_reason` fields (with doc comments) to a `Row` struct, a new
`default_status` function, and in `read_existing_rows` hoists `record.get(4).unwrap_or("")
.to_string()` out of the struct literal into `let promoted_to = ...;` (after line 168), so the
field becomes the shorthand `promoted_to,` (after line 178); `status,` and `rejection_reason: ...`
are appended (after lines 180-181).

## Mapping

All 85 mismatches (54 visible, 31 invisible) are one off-by-one alignment inside the `Row { ... }`
literal of `read_existing_rows`, reason `APTED("qualified_name")` throughout, including the
after-side
`Insert (with children)` lines.

- Ground truth: before line 154 `promoted_to: record.get(4).unwrap_or("").to_string()` splits
  into the shorthand `promoted_to` (after line 178) and the value moved to the new `let` (after
  line 168); before line 155 `dataset: ...` is identical to after line 179; after line 181
  `rejection_reason: record.get(7).unwrap_or("").to_string()` is new.
- omnidiff: the tree edit distance pairs the field initializers in order of kind: before 154
  (`promoted_to: ...get(4)...`) with after 179 (`dataset: ...get(5)...`) and before 155
  (`dataset: ...get(5)...`) with after 181 (`rejection_reason: ...get(7)...`), since both
  are `field_initializer` while the shorthand `promoted_to` is a different kind. The hoisted value
  is not recognised as moved, so the new `let` line's value is inserted, and the unchanged
  `dataset` field is reported as two updates. `matcher`: the `field_initializer` /
  `shorthand_field_initializer` split and the hoist across subtrees are not detected.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper
began printing them.

## Painting

Matcher-caused (Minimal 226 of 230 real bytes; Full 258 of 270): all on before lines 154-155 and
after lines 168, 178-181, the direct rendering of the mapping above: `Update -> None` on the
unchanged `dataset` line on both sides (47 bytes each in Minimal, 61 in Full) and on
`promoted_to:`; `Insert -> Move` on the hoisted value on after line 168 (40 bytes); `Delete ->
Move` on its old tail; `Insert -> None` on `promoted_to,`; `Update -> Insert` on the
`rejection_reason` line.

Renderer-caused (present with the human mapping too):

- Minimal, 4 bytes: the painting's `Delete` of `: ` after `promoted_to` (before line 154) and
  `Insert` of the `;` ending the new `let` (after line 168) are lone structural punctuation, which
  Minimal drops; the painting's `Move` on after line 168 also starts at the space before `record`,
  which Minimal trims. `renderer` (the leading space is against painting rule 4).
- Full, 12 bytes: the indentation of the new `let` line (8) is unpainted where the painting
  inserts it, the space after `:` / `=` beside the moved value, and the trailing `,` of the
  `promoted_to` fields painted `Move` (both sides). `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: APTED alignment of struct-literal fields (pair a `field_initializer` with
  a same-named `shorthand_field_initializer`, and the hoisted value with the new `let`), or a
  move pass after `APTED("qualified_name")` that rescues the `dataset` field identical in both
- Ground truth questionable: no (one Minimal `Move` range on after line 168 starts with a space)
