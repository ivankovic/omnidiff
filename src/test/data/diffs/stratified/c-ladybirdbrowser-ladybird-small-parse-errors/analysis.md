# Accuracy analysis: c-ladybirdbrowser-ladybird-small-parse-errors

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (0 visible) of 9043 nodes, 9039 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 31576: renderer 2, matcher 2 |
| Painting, Full | 8 bytes wrong of 31576: renderer 4, matcher 4 |
| Ground-truth invariant violations | none |

## The change

ladybird `Libraries/LibWasm/AbstractMachine/Validator.h` (C++ parsed with the C grammar): the
member `Module const* current_module { nullptr };` (before line 40) is removed. The C grammar
does not know brace member initialisers (`{ 0 };`), so the struct's last lines parse into
`ERROR` nodes. `description.md`: "Impossible to map because of parse errors. Painting is
correct."

## Mapping

- **One `ERROR` node, invisible** (before line 39, around `{ 0 }`): ground truth maps it to the
  after side's `ERROR` (`MatchButNotIdentical`); omnidiff deletes it (`APTED("qualified_name")`).
  The `ERROR` nodes group the tokens of lines 39-41 differently on the two sides because a member
  was removed from the middle of the error region. `parse-error`.

## Painting

The painting deletes only before line 40. All runs are `}` / `;` / `};` tokens at the end of the
struct (before lines 39-41, after lines 39-40) whose pairing depends on how the `ERROR` nodes
grouped them.

- **Minimal, 4 bytes:** `};` on before line 41 painted Delete (also with the human mapping) and
  `};` on after line 39 painted Insert (matcher-only). `parse-error`.
- **Full, 8 bytes:** the same two plus `};` on before line 39 / after line 40 painted Move (the
  human mapping gives `;` Move there, 2 bytes). `parse-error`.

## Verdict

- Primary cause: `parse-error`
- Other causes: none
- Where a fix would go: none in the matcher (C++ header parsed as C); a C++ grammar for `.h` files
  with C++ content would remove it
- Ground truth questionable: no
