# Accuracy analysis: typescript-add-generics

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 223 nodes, 223 graded, N:M floor 0 visible |
| Painting, Minimal | 58 bytes wrong of 655: renderer 52, matcher 24 |
| Painting, Full | 28 bytes wrong of 655: renderer 4, matcher 24 |
| Ground-truth invariant violations | 4 (rule [16], Minimal painting of `NumberContainer` -> `Container`, lines 1 and 17) |

## The change

A handmade fixture: `class NumberContainer` becomes the generic `class Container<T>`, every
`number` type inside becomes `T`, and the single `const container = new NumberContainer(42);`
becomes `numberContainer = new Container<number>(42)` plus a new `stringContainer` line.

## Mapping

- Before line 1, the class name `NumberContainer`, 1 visible. The ground truth updates it to
  `Container`. omnidiff deletes it (reason `APTED("fast_fallback")`, the end-of-pipeline APTED
  fallback) and paints the after `Container` as inserted, although the rest of the class is
  matched. No earlier pass paired the name (the class's own name is what changed), and the
  fallback did not pair the two names either. `matcher`.

## Painting

Matcher-caused, 24 bytes in each preset: line 1, `NumberContainer` Delete (15) and `Container`
Insert (9) where the paintings have an Update.

Minimal, the rest (34 bytes, same with the human mapping):

- Line 17, `NumberContainer` -> `Container` (18 bytes: `Container` on each side): omnidiff's
  Minimal rendering narrows the Update to the differing word `Number` and leaves `Container`
  unpainted; the Minimal painting updates the whole identifier. `invariants.txt` flags exactly
  this (rule [16]) for lines 1 and 17 on both sides: for a Minimal reading only `Number` differs.
  `ground-truth`. (On line 1 the same 18 bytes are hidden inside the matcher's run.)
- Line 17, `container` -> `numberContainer` (16 bytes: `ontainer` on each side): omnidiff narrows
  the update, the painting updates the whole identifier. Not flagged by the invariant check.
  `renderer`.

Full, the rest (4 bytes, same with the human mapping): the two spaces of ` = new ` on line 17,
which shifted right by six columns: the Full painting matches ` = new ` as moved, omnidiff paints
the tokens Move but not the spaces between them. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `ground-truth`, `renderer`
- Where a fix would go: pairing a renamed class name in `apted::for_roots_fallback` (or the pass
  that matched the class body); word-level narrowing in `diff::text` for Minimal.
- Ground truth questionable: yes: the Minimal painting updates all of `NumberContainer` ->
  `Container` on lines 1 and 17, which the invariant check (rule [16]) reports as wrong for a
  Minimal reading.
