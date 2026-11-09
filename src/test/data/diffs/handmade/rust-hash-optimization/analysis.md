# Accuracy analysis: rust-hash-optimization

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 4 (4 visible) of 573 nodes, 559 graded, N:M floor 0 visible |
| Painting, Minimal | 135 bytes wrong of 1538: renderer 58, matcher 117 |
| Painting, Full | 127 bytes wrong of 1538: renderer 54, matcher 121 |
| Ground-truth invariant violations | none |

## The change

Handmade. `SubtreeKey` gets a precomputed `hash` and `len` and a manual `Hash` impl, and `main`
keys its memo by `(SubtreeKey, SubtreeKey)` instead of `(Vec<usize>, Vec<usize>)` (line 16 /
33): the tuple `(before_subtrees.clone(), after_subtrees.clone())` (line 20) becomes
`(before_key.clone(), after_key.clone())` (after line 39), with `before_key` / `after_key` built
from the old vectors on two new lines (after lines 37-38).
`description.md`: "Requires a complex N->1 mapping."

## Mapping

Four visible mismatches, one cause, reason `APTED("qualified_name")`. The ground truth moves
`before_subtrees` and `after_subtrees` from the tuple on line 20 into the new
`SubtreeKey::new(&before_subtrees)` / `SubtreeKey::new(&after_subtrees)` calls (after lines
37-38) and inserts `before_key` / `after_key` in the tuple. omnidiff keeps the tuple's elements in
place and renames them, `before_subtrees` -> `before_key` and `after_subtrees` -> `after_key`
(`Update`), and inserts the new calls' arguments. The positional update is cheaper for the tree
edit distance than a move into a new statement, and no move pass reclaims the identical
identifiers. `matcher`.

## Painting

Matcher-caused (Minimal ~77 of the 117 matcher bytes, Full similar): the mapping above, rendered:
`before_subtrees.clone(),` / `after_subtrees` on line 20 painted as an update where the painting
has them moved, the arguments on after lines 37-38 painted `Insert` (painting `Move`), and
`before_key.clone(),` / `after_key` on after line 39 painted as an update (painting `Insert`).

Ungraded nodes (40 bytes Minimal, 44 Full): the two `Vec<usize>` type arguments on line 16 and
`SubtreeKey` on after line 33. The ground truth's tree does not list them (`graded` 559 of 573
nodes), while its painting marks them `Update`. omnidiff deletes the generic types and inserts the
type identifiers (`Delete`/`Insert -> Update`); rendering the sparse human mapping paints nothing
there, which is why the census splits these bytes between renderer and matcher. Replacing
`Vec<usize>` by `SubtreeKey` can be read as an update or as delete+insert. `ambiguous`.

Renderer-caused, with the human mapping only: `.clone(),` behind the moved identifiers (line 20,
after line 39, 18 bytes Minimal) slides left once the identifier is moved out and is painted
`Move`; in Full, `);` closing the tuple (4 bytes), the inserted indentation of after line 37 and
two single spaces. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `ambiguous`, `renderer`
- Where a fix would go: a move pass after `APTED("qualified_name")` that prefers moving an
  identical identifier into a new statement over renaming it in place when the new name is also
  inserted nearby
- Ground truth questionable: no (the tree leaves the replaced type arguments ungraded while the
  painting updates them; worth a look, not a contradiction)
