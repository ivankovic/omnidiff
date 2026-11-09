# Accuracy analysis: java-defects4j-jsoup-16-documenttype

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 4 (4 visible) of 700 nodes, 700 graded, N:M floor 0 visible |
| Painting, Minimal | 25 bytes wrong of 2596: renderer 4, matcher 21 |
| Painting, Full | 107 bytes wrong of 2596: renderer 86, matcher 21 |
| Ground-truth invariant violations | none |

## The change

`DocumentType` validates its name (new import line 4, `Validate.notEmpty(name);` line 22 after) and
writes the real doctype name: `accum.append("<!DOCTYPE html");` (line 33) becomes
`accum.append("<!DOCTYPE ").append(attr("name"));` (line 35 after), and the `' '` before the system
id becomes `" \""` (line 37 -> 39).

## Mapping

- Line 33 -> 35, the `.` and `append` of `accum.append(..)`: tree-sitter makes the after statement
  `(accum.append("<!DOCTYPE ")).append(attr("name"))`, with the new `.append` outermost. The ground
  truth keeps the before `.append` with the inner call `accum.append(` (unchanged text) and treats
  the outer `.append` (column 35) as new, matching its painting, which inserts
  `.append(attr("name"))`. omnidiff (`APTED("qualified_name")`) maps the before call to the after
  outer call and so its `.`/`append` to the new outer ones, and inserts the inner pair. 4 visible.
  The ground truth's reading is the natural one for an appended call. `matcher`.

## Painting

Matcher-caused (absent with the human mapping), both presets, 21 bytes on line 33/35: the before
`.append` painted Move (7), the after inner `.append` painted Insert (7) and the after outer
`.append` painted Move (7), where the paintings leave the first two alone and insert the third.
`matcher`.

Renderer-caused (also with the human mapping):

- Both presets, `Move -> None`, 4 bytes: the `")` closing the shortened string on line 33/35 is
  painted Move, because the deleted `html` shifts it left and the row carries several edits, so
  neither the prefix/suffix test nor the same-column uniqueness test exempts it. `renderer`.
- Full only, `Move -> None`, 82 more bytes: the unchanged tail
  `).append(attr("systemId")).append("\"");` on line 37/39 (40 bytes each side), plus `;` on line
  33/35. Minimal exempts the tail as the row's identical suffix; Full's
  `shifted_by_an_edit_beside_it` requires `s.start_row == d.start_row`, and the two lines inserted
  above shift the row index by 2. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`
- Where a fix would go: src/diff/text.rs identical_or_move: Full gating of
  shifted_by_an_edit_beside_it on s.start_row == d.start_row; APTED fallback (qualified_name) for
  appended calls
- Ground truth questionable: no
