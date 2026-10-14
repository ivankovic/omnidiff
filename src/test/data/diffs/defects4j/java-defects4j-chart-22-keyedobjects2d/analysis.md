# Accuracy analysis: java-defects4j-chart-22-keyedobjects2d

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 23 (16 visible) of 4382 nodes, 4382 graded, N:M floor 0 visible |
| Painting, Minimal | 219 bytes wrong of 29772: renderer 117, matcher 102 |
| Painting, Full | 308 bytes wrong of 29772: renderer 168, matcher 158 |
| Ground-truth invariant violations | 6 |

## The change

Defects4J Chart-22 in `KeyedObjects2D`. In `getObject(Comparable, Comparable)` (before lines
231-237) the `KeyedObjects rowData = ...` declaration moves out of `if (row >= 0) {`, a new `int
index = rowData.getIndex(columnKey);` is added, and the guard becomes `if (index >= 0)` with
`getObject(index)`. `removeColumn` gains a scan that drops an all-null column, `removeRow` gains an
`UnknownKeyException` guard, and another `removeColumn` checks the index before `removeValue`.

## Mapping

- **The `if (row >= 0)` statement in `getObject` (16 visible, 7 invisible).** Before lines 231-234:
  `if`, the condition `(row >= 0)`, the block braces and `return rowData.getObject(columnKey);`.
  The ground truth maps them to after lines 233-235 (`row` -> `index` and `columnKey` -> `index`
  as Updates; the rest Identical). omnidiff leaves all of them unmapped, op Delete, reason
  `APTED("large_flat_subtree")`; the after `if` is inserted. The `else { return null; }` of the
  same `if` is still matched, so the fallback split one statement in two. The `if` lost its
  first child (the declaration that moved out) and two identifiers were renamed, which is
  apparently enough for the flat-subtree edit distance to prefer delete+insert. `matcher`.

## Painting

Matcher-caused (absent with the human mapping):

- Minimal 102 bytes, Full 158 bytes, Delete/Insert -> None: the `if (row >= 0) {`,
  `return rowData.getObject(columnKey);` and `}` of before lines 231-234 painted Delete, and
  `if (index >= 0) {`, `return rowData.getObject(index);`, `}` of after lines 233-235 painted
  Insert. The painting leaves the kept tokens unpainted and paints only the renamed identifiers.

Renderer-caused (also with the human mapping):

- Minimal 114 bytes, Full 130 bytes, None -> Move: `KeyedObjects rowData = (KeyedObjects)
  this.rows.get(row);` (before line 232, after line 231). Both mappings pair it; it left the `if`
  block for the method body. Its row order relative to its neighbours did not change (the `if`
  line above it was removed), so the renderer does not paint it `Move`; the painting does.
  `renderer`.
- Full 12 bytes, Move -> None: `>= 0) {` on before line 231 / after line 233 and the `);` after a
  renamed argument (before lines 233 and 378, after lines 234 and 406). They keep their text and
  are displaced by the rename beside them; `FULL`'s `paint_displaced_moves` paints them `Move`,
  the Full painting does not. `renderer`.
- Full 16 bytes, Insert -> None: the 8-space indentation of the whitespace-only after lines 320
  and 330, inside the inserted block. The Full painting leaves those lines unpainted. `renderer`.

Painting-caused:

- Minimal 3 bytes, None -> Insert: a trailing space at the end of after lines 321, 332 and 368.
  The Minimal painting's inserted runs end on that space; omnidiff trims it. `invariants.txt`
  flags exactly these runs (6 violations, the same 3 rows in both paintings, "ends its last
  painted run on ' '"), against the rule that whitespace at the ends of a range is never
  painted. `ground-truth`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`, `ground-truth`
- Where a fix would go: `APTED("large_flat_subtree")` (`solve_large_flat_subtrees`), which deletes
  an `if` whose `else` it keeps matched; `diff::text` move detection for a statement that changed
  parent without changing row order
- Ground truth questionable: yes: the paintings end three inserted runs on a trailing space (after
  lines 321, 332, 368), which `invariants.txt` reports as 6 violations
