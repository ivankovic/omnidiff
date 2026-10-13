# Accuracy analysis: java-defects4j-jxpath-18-attributecontext

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 45 (34 visible) of 1156 nodes, 1156 graded, N:M floor 0 visible |
| Painting, Minimal | 274 bytes wrong of 6447: renderer 112, matcher 162 |
| Painting, Full | 396 bytes wrong of 6447: renderer 174, matcher 265 |
| Ground-truth invariant violations | none |

## The change

Defects4J JxPath-18. `AttributeContext.nextNode()` used to bail out unless `nodeTest` was a
`NodeNameTest`. The fix adds a `WILDCARD_TEST` constant and replaces the guard with an `if
(NodeTypeTest) { ... } else if (NodeNameTest) { ... }` chain that computes `nodeNameTest`, then `if
(nodeNameTest == null) { return false; }`. The `iterator = ...attributeIterator(name)` statement is
reflowed and now calls `nodeNameTest.getNodeName()`.

## Mapping

All 45 mismatches (34 visible) come from one whole-method APTED call, reason
`APTED("qualified_name")` (the qualified-name group match in `solve_syntax_aware_matching`).

- **Guard mapped to the wrong `if` (20 visible, 9 invisible), `matcher`.** The ground truth deletes
  the old guard `if (!(nodeTest instanceof NodeNameTest)) {` (line 75) and its `}` (line 77), and
  maps its `return false;` (line 76) into the new `if (nodeNameTest == null)` of line 87 (it sits on
  line 88). omnidiff maps the old `if_statement`, both parenthesized expressions, the block, the
  `if`/`(`/`(`/`)`/`)`/`{`/`}` tokens and `nodeTest` to the new nested `if (((NodeTypeTest)
  nodeTest).getNodeType() == Compiler.NODE_TYPE_NODE) {` at lines 80-82. The old `;` goes to
  `nodeNameTest = WILDCARD_TEST;` (line 81), and `return`/`false` become Delete. The two `if`s share
  only punctuation and one identifier. APTED preserves order, and it keeps the old cast (next group)
  on line 85. That cast comes after the old guard, so the guard cannot also reach the `if
  (nodeNameTest == null)` on line 87, which comes after line 85. This reading is consistent with the
  output; the cost trace was not inspected.
- **`QName name` renamed across kinds (4 visible), `matcher`.** `QName` (type_identifier) and `name`
  (identifier) on line 78 are mapped MatchButNotIdentical to `nodeTest` (identifier) and
  `NodeNameTest` (type_identifier) of `else if (nodeTest instanceof NodeNameTest)` on line 84. The
  ground truth deletes and inserts them. Neither the kinds nor the roles agree.
- **Identical cast carried over (8 visible, 2 invisible), `ambiguous`.** `(NodeNameTest) nodeTest`
  on line 78 (inside `((NodeNameTest) nodeTest).getNodeName()`) is mapped Identical to the cast in
  `nodeNameTest = (NodeNameTest) nodeTest;` on line 85. The ground truth deletes it and inserts it
  again. The text is byte-identical and does the same job, so keeping it is defensible.
- **`name` -> `nodeNameTest` (2 visible), `ambiguous`.** The old argument `name` (line 80) is
  updated to the receiver `nodeNameTest` of `nodeNameTest.getNodeName()` (line 91). The ground truth
  deletes `name` and inserts the whole call. omnidiff reads it as a renamed argument wrapped in a
  call. Both readings are defensible.

## Painting

Matcher-caused runs (absent when the human mapping is rendered): the guard tokens on line 75 and `}`
on line 77 are Move/None where the painting has Delete. `return false` is Delete/Insert where
Minimal leaves it unpainted. `QName name ` is Update and the old cast is Move where the painting has
Delete. On the after side, the nested `if` header and body tokens (lines 80-82) and the else-if line
84 are Move/Update/None where the painting has Insert. `nodeNameTest` on line 91 is None or Update
where it is Insert. This is about 162 bytes in Minimal and about 220 bytes of Full's own runs.
`matcher`.

Renderer-caused runs (they survive with the human mapping):

- `parentContext.getCurrentNodePointer().attributeIterator(` (line 80 before, line 90 after; 56
  bytes per side, plus `);` in Full) is painted Move. The statement was only reflowed (`iterator =`
  joined onto the call's line). The painting leaves the unchanged text unpainted. This is 112 bytes
  in Minimal and about 116 in Full. `renderer`.
- Full only: `return false;` (line 76 before, line 88 after, 29 bytes per side) is Move in the Full
  painting. Rendering the human mapping leaves it unpainted, though the human mapping moves it into
  a different `if`. 58 bytes. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`, `ambiguous`
- Where a fix would go: APTED alignment under
  `solve_syntax_aware_matching::solve_qualified_name_groups` (the guard and `QName name` groups);
  `src/diff/text.rs` Move painting of reflowed identical text, and Full Move painting of a statement
  re-parented into another `if`
- Ground truth questionable: no
