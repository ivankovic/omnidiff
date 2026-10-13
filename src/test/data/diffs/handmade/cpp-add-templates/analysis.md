# Accuracy analysis: cpp-add-templates

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 224 nodes, 224 graded, N:M floor 0 visible |
| Painting, Minimal | 18 bytes wrong of 493: renderer 18, matcher 0 |
| Painting, Full | 10 bytes wrong of 493: renderer 10, matcher 0 |
| Ground-truth invariant violations | 9 (six rule 16 in Minimal, three rule 18 in the tree mapping) |

## The change

Handmade C++ fixture (no provenance README): `class IntBox` becomes `template<typename T> class
Box`. `int` becomes `T` in the field, the constructor parameter and the getter's return type, the
constructor is renamed `Box`, and `main` uses `Box<int> box(42);`.

## Painting

The mapping matches the ground truth exactly, and every run is identical with the human mapping.

- Minimal, `None -> Update`, 18 bytes, `Box` of `IntBox` / `Box` on before lines 3, 6, 11 and after
  lines 4, 7, 12: the Minimal painting updates the whole identifier `IntBox` -> `Box`; omnidiff
  narrows the update to the dropped `Int` and leaves the shared `Box` unpainted. invariants.txt
  flags all six painting entries (rule 16: in a Minimal reading only the differing word `Int`
  should be painted). `ground-truth`.
- Full, `None -> Move`, 8 bytes, the space after a replaced type (`int value`, `int v`,
  `int getValue`, `IntBox box` -> `T value`, `T v`, `T getValue`, `Box<int> box`; before lines 4, 6,
  7, 11, after lines 5, 7, 8, 12): the Full painting matches the space as a Move; omnidiff leaves
  it unpainted. `renderer`.
- Full, `Move -> None`, 2 bytes, the class body `{` on before line 3 / after line 4: shifted left
  by the shorter name, and omnidiff paints it Move. The painting leaves it unpainted. `renderer`.

## Verdict

- Primary cause: `ground-truth`
- Other causes: `renderer`
- Where a fix would go: ground truth (Marko's call) for the Minimal `IntBox` -> `Box` entries; Full
  gap-whitespace and displaced-bracket rules in `identical_or_move` (`src/diff/text.rs`)
- Ground truth questionable: yes: invariants.txt flags the six Minimal `IntBox` -> `Box` entries
  (rule 16). It also flags three tree-mapping entries (rule 18) that delete `int` and insert `T` in
  the type slot of matched parents (field, parameter, return type). omnidiff agrees with those
  entries, so they cost nothing here, but the two are the same element by that rule.
