# Accuracy analysis: cpp-libreoffice-fraction-type-to-double

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 30192 nodes, 30192 graded, N:M floor 0 visible |
| Painting, Minimal | 28 bytes wrong of 114156: renderer 0, matcher 28 |
| Painting, Full | 32 bytes wrong of 114156: renderer 4, matcher 32 |
| Ground-truth invariant violations | none |

## The change

In two places (lines 219-220 and 886-888), `const Fraction aUIScale(...)` becomes
`const double fUIScale(...)`, and the variable's uses are renamed.

## Mapping

- **Cross-kind type update (2 visible, `matcher`, `APTED("qualified_name")`).** The ground truth
  maps `Fraction` (`type_identifier`) to `double` (`primitive_type`) as a match in both declarations
  (lines 219 and 886). Omnidiff deletes `Fraction` and inserts `double`. The declarations around
  them, and the rename `aUIScale` -> `fUIScale`, are matched. The two type nodes have different
  kinds, and APTED's cost keeps them apart.

## Painting

- **Matcher (Minimal 28, Full 32 bytes, `Delete/Insert -> Update`).** `Fraction` and `double` are
  painted Delete and Insert where both paintings have Update. In Full the runs include the space
  after the type.
- **Renderer (Full, 4 bytes, `Update -> Move`).** With the human mapping, omnidiff's Full rendering
  extends the Update over the space after the type. The Full painting labels that space Move.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: APTED's label cost. It would let `type_identifier` and `primitive_type` in a
  declaration's type slot be updated into each other.
- Ground truth questionable: no
