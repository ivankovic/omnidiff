# Accuracy analysis: csharp-jellyfin-jellyfin-small-add

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 4 (3 visible) of 3655 nodes, 3655 graded, N:M floor 0 visible |
| Painting, Minimal | 15 bytes wrong of 16323: renderer 1, matcher 14 |
| Painting, Full | 111 bytes wrong of 16323: renderer 109, matcher 28 |
| Ground-truth invariant violations | none |

## The change

`Playlist.GetRecursiveChildren` gains an `out int totalCount` parameter (line 152). Its body
`return GetPlayableItems(user, query);` (line 154) becomes `var items = GetPlayableItems(user,
query);`,
`totalCount = items.Count;` and `return items;` (after lines 154-156).

## Mapping

Four mismatches (3 visible), all `APTED("qualified_name")`, on line 154. The ground truth keeps
`return` and its `;` on the new `return items;` (after line 156). The call moves into the new
declaration, and that declaration's `;` is new. Omnidiff deletes `return` and the
`return_statement`, maps the old `;` to the new declaration's `;` (after line 154), and inserts
`return items;`. APTED keeps the call and its statement terminator together instead of the
return statement. `matcher`.

## Painting

- Minimal, matcher, 14 bytes: `return` painted Delete (before line 154), and `return ` and `;`
  painted Insert (after line 156), where the painting leaves them alone. This is all but one byte
  of Minimal's disagreement.
- Minimal, renderer, 1 byte, `None -> Insert`: the `;` that ends the new `var items = ...` line.
  It stands alone in its range, so the Minimal punctuation filter drops it.
- Full, renderer, about 83 bytes, also present with the human mapping. The call
  `GetPlayableItems(user, query)` moved from the return (column 19) into the new declaration
  (column 24), and the Full painting paints it, with the indentation and `return`, as Move.
  Omnidiff leaves it unpainted. The new line's indentation is not painted Insert either.
  `renderer`.
- Full, matcher, 28 bytes: `            return` and `;` of the new `return items;` painted Insert
  where the Full painting moves them.

## Verdict

- Primary cause: `renderer` (by bytes, driven by Full; Minimal alone is the matcher's)
- Other causes: `matcher`
- Where a fix would go: the APTED call at `qualified_name` should keep a `return` whose value is
  extracted into a local. The Full Move painting of a call that moved from a return into a new
  declaration (`src/diff/text.rs`) is the other part.
- Ground truth questionable: no
