# Accuracy analysis: typescript-add-type-annotations

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 176 nodes, 176 graded, N:M floor 0 visible |
| Painting, Minimal | 10 bytes wrong of 454: renderer 10, matcher 0 |
| Painting, Full | 226 bytes wrong of 454: renderer 226, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A handmade fixture: an `interface User` is added at the top (after lines 1-6), and type
annotations are added to `addUser(name: string, age: number): User` and to
`const user: User = ...`. The mapping is perfect; every painting run is the same with the human
mapping, so all are the renderer's.

## Painting

### Minimal, 10 bytes, Move -> None

`, age` in the `addUser` parameter list (before line 1, after line 7, 5 bytes each side): pushed
right by the inserted `: string`, and painted Move by omnidiff. The Minimal painting records only
the inserted annotations. `renderer`.

### Full, 226 bytes

- Move -> None, 216 bytes: the whole function body from `{` to `}` (before lines 1-7, after lines
  7-13) and `= addUser("Alice", 30);` (before line 9, after line 15), on both sides. The
  statement block starts at the `{` that the inserted `: User` pushed right, and omnidiff's Full
  rendering paints the whole multi-line block Move for that start shift; the same holds for the
  initializer pushed right by `: User`. The Full painting leaves both unpainted. `renderer`.
- None -> Move, 10 bytes: `(name` on both sides. The Full painting matches the parameter list's
  pieces (`(`, `name`, `,`, ` `, `age`, `)`) as moved; omnidiff does not paint `(` and `name`,
  which kept their column. `renderer` (the painting's own choice here is a style judgement).

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` displacement handling: a same-row horizontal shift should not
  paint a whole multi-line block (Full) or a shifted parameter (Minimal) as Move.
- Ground truth questionable: no
