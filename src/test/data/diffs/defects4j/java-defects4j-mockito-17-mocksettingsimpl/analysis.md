# Accuracy analysis: java-defects4j-mockito-17-mocksettingsimpl

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 1089 nodes, 1089 graded, N:M floor 0 visible |
| Painting, Minimal | 31 bytes wrong of 4736: renderer 3, matcher 28 |
| Painting, Full | 31 bytes wrong of 4736: renderer 5, matcher 28 |
| Ground-truth invariant violations | none |

## The change

Defects4J Mockito-17. `MockSettingsImpl` gets a `serializable` flag. `serializable()` sets it and
`return this;` instead of adding `Serializable` to the extra interfaces. `isSerializable()` returns
the flag instead of `extraInterfaces != null && Arrays.asList(extraInterfaces).contains(...)`.

## Mapping

- **`extraInterfaces` -> `serializable` (1 visible), `ambiguous`.** In `isSerializable()` the ground
  truth maps the first operand `extraInterfaces` (line 74) to the new return value `serializable`
  (line 76) as an Update. The rest of the old expression is deleted. omnidiff has no pass that
  decides the node, and the terminal sweep deletes it (`UnresolvedNode`), so `serializable` is
  inserted. The returned value changed from a derived check to a stored flag. Reading the identifier
  as renamed is the ground truth's choice. Reading it as a replacement is equally defensible: the
  two names refer to different fields of different types.

## Painting

- **Matcher-owned, 28 bytes in Minimal and 27 in Full (`Delete/Insert -> Update`).** These are
  `extraInterfaces` on line 74 and `serializable` on line 76. `ambiguous`.
- **Renderer, Minimal, 3 bytes.** The spaces between the separately deleted `!=`, `null` and `&&` on
  line 74 are painted Delete. The Minimal painting deletes the tokens one by one and leaves the
  spaces. `renderer`.
- **Renderer, Full, 5 bytes.** The `;` of `return this.extraInterfaces(...)` -> `return this;`
  (lines 22/24) and of the `isSerializable` return (lines 74/76) only shifted column. Full paints it
  Move, and the painting leaves it. One space before `!=` that the Full painting deletes is left
  unpainted under the human mapping. `renderer`.

## Verdict

- Primary cause: `ambiguous`
- Other causes: `renderer`
- Where a fix would go: none for the mapping (ambiguous); `src/diff/text.rs` for Full `;` Move on a
  column shift and Minimal interior spaces between deleted tokens
- Ground truth questionable: no
