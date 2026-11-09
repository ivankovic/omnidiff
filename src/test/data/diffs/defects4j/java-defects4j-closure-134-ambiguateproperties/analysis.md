# Accuracy analysis: java-defects4j-closure-134-ambiguateproperties

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 45 (29 visible) of 9047 nodes, 9047 graded, N:M floor 0 visible |
| Painting, Minimal | 175 bytes wrong of 41636: renderer 22, matcher 154 |
| Painting, Full | 273 bytes wrong of 41636: renderer 109, matcher 183 |
| Ground-truth invariant violations | none |

## The change

Closure-134: `AmbiguateProperties.Property` stops tracking a least-supertype `JSType type` and
keeps a bit set of related types instead. The field `JSType type;` (before line 553) is removed
and `JSTypeBitSet relatedTypesSet = ...;` added (after 556); `getRelated(prop.type)` becomes
`prop.relatedTypesSet` (lines 356, 366); in `addType`, `if (type == null) { type = newType; }
else { type = type.getLeastSupertype(newType); } typesSet.set(getIntForType(newType));` (before
594-599) becomes `int typeInt = ...; if (!typesSet.get(typeInt)) { computeRelatedTypes(newType);
typesSet.set(typeInt); relatedTypesSet.or(getRelatedTypesOnNonUnion(newType)); }` (after
593-598).

## Mapping

Reasons: all 43 mismatches come from `APTED("large_flat_subtree")`.

- The removed and the added field, 6 visible + 4 invisible: the ground truth deletes `JSType type;`
  (line 553) and inserts `JSTypeBitSet relatedTypesSet = new ...;` (line 556). omnidiff matches the
  two `field_declaration`s, updating `JSType` -> `JSTypeBitSet` and `type` -> `relatedTypesSet` and
  keeping `;` (`APTED("large_flat_subtree")`). Different fields at different places with no shared
  name. `matcher`.
- The rewritten `addType` body, 21 visible + 10 invisible. The ground truth moves
  `typesSet.set(getIntForType(newType));` (before 599) into the new block as
  `typesSet.set(typeInt);` (after 596). omnidiff instead pairs it with the new
  `relatedTypesSet.or(getRelatedTypesOnNonUnion(newType));` (after 597), updating
  `typesSet` -> `relatedTypesSet`, `set` -> `or`, `getIntForType` -> `getRelatedTypesOnNonUnion`
  (`APTED("large_flat_subtree")`), and gives the after `typesSet` to the before `type` of
  `type = type.getLeastSupertype(newType);` (line 597). The identical receiver and method name
  `typesSet.set(` were available; APTED chose by shape (a call with one nested call argument).
  `matcher`.
- The `if` block braces, 2 visible: the ground truth keeps the `{` on before line 594 and the
  then-block's `}` on line 596 as the new `if`'s block braces (after 593, 598); omnidiff deletes
  them (op Delete, `APTED("large_flat_subtree")`). `matcher`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper
began printing them.

## Painting

- Matcher, Minimal 154 bytes / Full 183: the field pair painted as kept/Update instead of
  Delete + Insert (`JSType type;`, `JSTypeBitSet relatedTypesSet` ... `;`); before line 599's
  `typesSet.set(getIntForType(newType))` painted Update where the painting has Move + Delete;
  after line 597's new statement painted Update instead of Insert, and `typesSet.set(` / `);` on
  after 596 painted Insert or left unpainted instead of Move; the deleted then-block braces.
  `matcher`.
- Renderer, Minimal 22 bytes (survive with the human mapping): `getRelated(prop.type)` ->
  `prop.relatedTypesSet` on lines 356 and 366: omnidiff paints the kept `prop.` Move (it moved
  out of the argument list), the paintings leave it unpainted (20 bytes); and the `)` of
  `typesSet.set(...)` on before 599 / after 596, Move in the painting, unpainted by omnidiff.
  `renderer`.
- Renderer, Full 109 bytes: the unchanged tails `.intersects(typesInSet);` and `);` after the
  edit on lines 356/366 painted Move (52 bytes); the Full painting's Matches of `      if (`
  (same row and column, graded Move), `) {` and `}` lines of the rewritten `if`, and indentation,
  which omnidiff leaves unpainted. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: the `large_flat_subtree` APTED pass
  (`src/diff/solve_large_flat_subtrees.rs`): prefer statements sharing receiver and method
  (`typesSet.set(`) over shape-only matches, and do not pair unrelated field declarations.
- Ground truth questionable: no.
