# Accuracy analysis: rust-error-handling

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 5 (4 visible) of 301 nodes, 301 graded, N:M floor 0 visible |
| Painting, Minimal | 27 bytes wrong of 698: renderer 0, matcher 27 |
| Painting, Full | 34 bytes wrong of 698: renderer 20, matcher 30 |
| Ground-truth invariant violations | none |

## The change

Handmade. `read_file` and `main` switch from `.unwrap()` to `?` and return `Result`: line 7
`file.read_to_string(&mut contents).unwrap();` becomes `file.read_to_string(&mut contents)?;`,
and the tail expression `contents` (line 8) becomes `Ok(contents)`.

## Mapping

One group, reason `APTED("qualified_name")` (4 visible, 1 invisible), lines 7-8. The ground truth
keeps line 7's statement (`unwrap` deleted, `;` kept) and maps the tail `contents` into
`Ok(contents)`, with `Ok` inserted. omnidiff instead pairs the old `... .unwrap()` call with the
new `Ok(contents)` call, so `unwrap` (a `field_identifier`) is mapped to `Ok` (an `identifier`)
as `MatchButNotIdentical`; with the old call taken, line 7's `expression_statement` and its `;`
are deleted, and the tail `contents` is deleted and the new one inserted. A cross-statement
pairing by tree edit cost after the `.unwrap()` -> `?` rewrite. `matcher`.

## Painting

Matcher-caused (all 27 Minimal bytes, 30 of Full's): `unwrap` painted `Update` instead of
`Delete`, `Ok` `Update` instead of `Insert`, the old and new `;` on line 7 `Delete`/`Insert`
where they are unchanged, and `contents` on line 8 `Delete`/`Insert` where the painting has it
unchanged (Minimal) or moved (Full).

Renderer-caused (Full only, 20 bytes, with the human mapping too):

- `None -> Move`, `contents` on line 8 (8 bytes per side): the Full painting moves it into
  `Ok(...)`; it stays on its row index and is exempted as displaced by an edit beside it
  (`shifted_by_an_edit_beside_it` in `identical_or_move`), so it is not painted.
- `None -> Insert`, the 4-byte indentation in front of `Ok` on after line 8: the Full painting
  includes it in the `Insert`, but the line is not a wholly inserted one.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: APTED call-expression pairing (`APTED("qualified_name")`) when `.unwrap()`
  becomes `?` and a tail expression is wrapped in `Ok(...)`
- Ground truth questionable: no
