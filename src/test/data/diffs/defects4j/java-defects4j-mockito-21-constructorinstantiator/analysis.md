# Accuracy analysis: java-defects4j-mockito-21-constructorinstantiator

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 7 (5 visible) of 894 nodes, 894 graded, N:M floor 0 visible |
| Painting, Minimal | 111 bytes wrong of 3839: renderer 95, matcher 30 |
| Painting, Full | 120 bytes wrong of 3839: renderer 98, matcher 28 |
| Ground-truth invariant violations | 3 (rule 16 twice, rule 18 once) |

## The change

Defects4J Mockito-21. `ConstructorInstantiator.withOuterClass(cls)` becomes `withParams(cls,
Object... params)`. Instead of looking up one constructor by the outer class instance's type, it
loops `for (Constructor<?> constructor : cls.getDeclaredConstructors())`, checks the parameter types
with a new `paramsMatch` helper, and throws when nothing matches.

## Mapping

- **For header partly unmapped (5 visible, 2 invisible), `matcher`.** The ground truth maps the old
  `Constructor<T> c = cls.getDeclaredConstructor(outerClassInstance.getClass());` (line 24) to the
  new for header `for (Constructor<?> constructor : cls.getDeclaredConstructors())` (line 24): `c`
  -> `constructor` (Update), and the `method_invocation` with its `cls`, `.`, `(` and `)`. With
  `APTED("qualified_name")`, omnidiff keeps the type `Constructor<...>` and the method name
  (`getDeclaredConstructor` -> `getDeclaredConstructors`, Update). It deletes `c`, the invocation
  and argument-list nodes, and their `cls`, `.`, `(`, `)`, and inserts the new ones. A local
  declaration became a loop variable, a cross-kind change. The ground truth's pairing keeps the
  receiver and the parentheses with the method name they belong to. omnidiff's mapping keeps the
  name and drops its own call.

## Painting

Matcher-owned: `c ` / `constructor ` and ` cls.`, `(`, `)`, `s()` on line 24 are painted
Delete/Insert where the paintings match them. My byte count is about 18; the official matcher share,
which compares the two renderings, is 30 in Minimal and 28 in Full. `matcher`.

Renderer-owned (they survive with the human mapping):

- **Minimal, about 93 bytes.** Minimal paints Move on identical tokens that were pushed right or
  re-parented: `<T> T` (line 20, after the inserted `static`), `Constructor<` and `>` (line 24,
  after the inserted `for (`), `return` and `.newInstance(` (line 25 -> 27). The Minimal painting
  leaves all of these unpainted. The rename `withOuterClass` -> `withParams` (lines 17 and 20) is
  split with a shared trailing `s` left unpainted, where the painting updates `OuterClass` ->
  `Params` whole. There are also single bytes: the for's closing `}` on line 29 and a space before
  `{` on line 24. `renderer`.
- **Full, about 96 bytes.** The Full painting records almost every byte of the changed lines as
  separate Match pieces: indentation, `private` at an unchanged position, single spaces, and the
  shifted `)` of the call on line 17. Matches of identical text are graded as Move. It also inserts
  the new 8- and 20-column indentation (line 27). The Full rendering leaves this unpainted, and
  paints `;` Move on lines 25/27. `renderer`. The in-place `private` reads as a painting choice more
  than a relocation.

Ground truth (invariants.txt, 3 violations): the Minimal painting paints `c` against `constructor`
on lines 24 and 25 (rule 16; 2 bytes here, where omnidiff's split leaves the shared `c` unpainted).
Inside the matched `type_arguments` on line 24, the mapping deletes `T` and inserts `?`, though
nothing else can occupy that position (rule 18). omnidiff agrees with the mapping there, so rule 18
costs no bytes. `ground-truth`.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`, `ground-truth`
- Where a fix would go: `src/diff/text.rs` (Minimal Move painting of identical tokens shifted by an
  insertion or re-parented; rename split keeping a shared suffix); `APTED("qualified_name")` keeping
  a method name without its call
- Ground truth questionable: yes: invariants.txt reports 3 violations (Minimal paints `c` against
  `constructor` twice; `T` -> `?` deleted/inserted inside matched type arguments)
