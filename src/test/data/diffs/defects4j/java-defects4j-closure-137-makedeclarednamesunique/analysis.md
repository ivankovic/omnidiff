# Accuracy analysis: java-defects4j-closure-137-makedeclarednamesunique

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 190 (129 visible) of 6119 nodes, 6119 graded, N:M floor 0 visible |
| Painting, Minimal | 2060 bytes wrong of 30832: renderer 1657, matcher 1593 |
| Painting, Full | 3425 bytes wrong of 30832: renderer 2862, matcher 1909 |
| Ground-truth invariant violations | 5 |

## The change

Closure-137: `MakeDeclaredNamesUnique.ContextualRenameInverter` becomes a `ScopedCallback`. It
now collects referenced names per scope (`referencedNames`, `referenceStack`, new
`enterScope`/`exitScope`), maps names to their reference nodes (`Map<String, List<Node>>`), and
does the renaming in a new `handleScopeVar(Var v)` instead of inside `visit`. The old `visit`
body (before lines 286-337) is largely taken apart: its renaming logic moves into
`handleScopeVar`/`exitScope` (after 273-335), and a new, short `visit` (after 342-358) only
records references. Marko's description.md: "I can't imagine a diff like this being accepted by
a human. This is a defects4j research artifact, far beyond what humans would actually write."

## Mapping

190 mismatches (129 visible); all 190 carry reason `APTED("qualified_name")`, including the 28
"Delete/Insert (with children)" lines.

- The old `visit` body against the new `visit`, 109 visible, about 51 invisible. `visit` is
  matched by qualified name, and APTED inside it pairs the old body's statements with the new
  body's: `if (var == null || var.isGlobal()) { return; }` (before 291-293) with
  `if (t.inGlobalScope()) { return; }` (after 344-346), `if (nameMap.containsKey(var))` with
  `if (NodeUtil.isReferenceName(node))`, `String newName = getOrginalNameInternal(...)` with
  `String name = node.getString();`, the `if (... TokenStream.isJSIdentifier(newName))` condition
  shell with `if (containsSeparator(name))`, and so on, updating every identifier. The ground
  truth deletes those old statements and inserts the new ones, and instead maps the parts that
  survive to where they went: e.g. `TokenStream.isJSIdentifier(newName)` (before 319) and its
  comments to `handleScopeVar` (after 317-318), `compiler.reportCodeChange();` and the comment
  blocks to `exitScope`/`handleScopeVar`. omnidiff deletes those (op Delete). APTED scoped to
  the name-matched method cannot see that the body moved into other methods, and pairs
  same-shaped but unrelated statements instead. `matcher`.
- `private static String getNameSuffix(String name, int index) { return name.substring(...); }`
  (before 274-278) against the new `public void enterScope(NodeTraversal t) {` header and its
  first statements (after 273-280), 18 visible, about 10 invisible: omnidiff pairs the removed
  helper's tokens with the new method's (`private` -> `public`, `String` -> `enterScope`, `name`
  -> `t`, braces, `;`), also under `APTED("qualified_name")`: APTED pairs the removed method with
  the added one of another name. The ground truth deletes one and inserts the other.
  `matcher`.
- `Map<Var, String>` -> `Map<String, List<Node>>` (before 244, after 247), 2 visible: the ground
  truth updates `Var` -> `String`, deletes `String` and inserts `List<Node>`; omnidiff also
  updates the second type argument `String` -> `List` (APTED("qualified_name")). Both readings
  are positional and defensible. `ambiguous`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began printing them.

## Painting

- Matcher, Minimal 1593 / Full 1909 bytes: with omnidiff's mapping the code the ground truth
  moves into `handleScopeVar`/`exitScope` is painted Delete on the before side and Insert on the
  after side where the painting has Move (about 500 bytes each way in Minimal, 570 in Full), and
  the statements omnidiff paired inside `visit` and `getNameSuffix`/`enterScope` are painted
  Move or Update where the painting deletes or inserts them. `matcher`.
- Renderer, Minimal 1657 bytes (with the human mapping, 1649 of them None -> Move): the comments
  and lines the ground truth moves but whose relative order a line diff keeps (the
  `// Merge any names ...` and `// If there isn't anything left ...` blocks, the `/** For the
  Var declared ...` Javadoc, `// Check if the new name is valid ...`,
  `compiler.reportCodeChange();`, the old `@Override public void visit(...)` header). The
  painting paints them Move; omnidiff's move attribution (`crossed_backwards`, see
  `research/data/quality/move_attribution.md`) flags other nodes as the movers and leaves these
  unpainted. `renderer`.
- Renderer, Full 2862 bytes: the same unpainted moves (1903 bytes None -> Move) plus Full's Move
  painting of unchanged, displaced text the painting leaves alone (920 bytes Move -> None).
  `renderer`.

The matcher and renderer shares overlap (the 2060 real Minimal bytes are less than their sum);
fixing the matcher alone would leave the renderer's move attribution as the larger remainder.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`, `ambiguous`
- Where a fix would go: the APTED("qualified_name") scope in
  `src/diff/solve_syntax_aware_matching.rs` (detect statements moved from a name-matched method
  into new methods before pairing the remainder by shape, and do not pair a removed method with an
  added method of another name and purpose); move attribution in
  `src/diff/text.rs`.
- Ground truth questionable: no. invariants.txt lists 5 painting-hygiene violations (a run
  ending on a space at after line 320, a renamed identifier not painted whole on line 26 in Full,
  a one-space gap at after line 319 in Full); none affects the analysis above.
