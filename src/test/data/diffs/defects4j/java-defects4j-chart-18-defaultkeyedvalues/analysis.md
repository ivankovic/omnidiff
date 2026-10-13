# Accuracy analysis: java-defects4j-chart-18-defaultkeyedvalues

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 4232 nodes, 4232 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 29871: renderer 0, matcher 2 |
| Painting, Full | 6 bytes wrong of 29871: renderer 1, matcher 5 |
| Ground-truth invariant violations | none |

## The change

Defects4J Chart-18 in `DefaultKeyedValues`: `removeValue(int)` drops the `if (index <
this.keys.size())` guard around `rebuildIndex()`, and `removeValue(Comparable)` replaces `return;`
(before line 335) with `throw new UnknownKeyException(...)` (after lines 333-334).

## Mapping

- **The `;` of `return;` and of the new `throw` (2 visible).** Before line 335 col 10, after line
  334 col 46. The ground truth deletes the `return` statement whole and inserts the `throw`
  whole. omnidiff pairs the two semicolons, op Identical, reason `APTED("large_flat_subtree")`:
  the tree-edit-distance fallback over the method's flat children matches the only leaf the two
  statements share. A `;` paired across a deleted and an inserted statement means nothing to a
  reader (the `IdenticalHash` docs say the same about leaves). `matcher`.

## Painting

Matcher-caused (absent with the human mapping), 2 bytes Minimal and 5 bytes Full:

- Minimal, None -> Delete/Insert: the two `;`. Being matched, they are not painted as part of the
  deleted `return;` and the inserted `throw`.
- Full, Move -> Delete/Insert: the same two `;`, painted `Move`; and None -> Delete on the
  `\t\t\t` indentation of before line 335, which the Full painting includes in the deleted
  statement.

Renderer-caused (also with the human mapping):

- Full, 1 byte, Insert -> None: the trailing space after `+ key ` at the end of after line 333,
  inside the inserted `throw`. omnidiff paints it with the multi-row insert; the Full painting
  stops at `key`. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: `APTED("large_flat_subtree")` (`solve_large_flat_subtrees`): do not keep
  a lone punctuation leaf matched when its deleted and inserted parents are otherwise unmatched
- Ground truth questionable: no
