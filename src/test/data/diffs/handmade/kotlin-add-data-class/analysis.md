# Accuracy analysis: kotlin-add-data-class

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 189 nodes, 189 graded, N:M floor 0 visible |
| Painting, Minimal | 14 bytes wrong of 423: renderer 14, matcher 0 |
| Painting, Full | 38 bytes wrong of 423: renderer 38, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Handmade fixture. A `data class Person(val name: String, val age: Int)` is added, and
`createPerson` returns `Person(name, age)` instead of the multi-line
`mapOf("name" to name, "age" to age)`, with return type `Person` instead of `Map<String, Any>`.

## Painting

The mapping is perfect, and every run is the same with the human mapping: all are the
renderer's.

- Both presets, moved identifiers (Minimal 14 bytes, about 16 bytes in Full with the adjacent
  `,`/space): `name` and `age` from `"name" to name` / `"age" to age` (before lines 3-4) are painted
  Move into `Person(name, age)` (after line 4) by both paintings. Omnidiff leaves them unpainted.
  Their parent `to` expressions are deleted, so the identifiers change parent, but they stay in
  order inside the matched `mapOf(` -> `Person(` call; the renderer does not show that as a move.
  `renderer`.
- Full only, indentation of removed lines (20 bytes): the Full painting paints the leading
  whitespace of before lines 3-5 as Delete; the rendering does not paint indentation.
  `renderer`.
- Full only, `{` (2 bytes): the opening brace of `createPerson`'s body (before line 1, after
  line 3) is painted Move by omnidiff, because the return type in front of it changed length; the
  Full painting leaves it unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: text renderer: Move painting for identical leaves that changed parent
  inside a matched call; whitespace of deleted lines under Full; no Move for a brace that only
  shifted.
- Ground truth questionable: no
