# Accuracy analysis: java-defects4j-closure-128-codegenerator

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 18736 nodes, 18736 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 78669: renderer 4, matcher 0 |
| Painting, Full | 50 bytes wrong of 78669: renderer 50, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-128: `CodeGenerator.isSimpleNumber` treats `"0"` as a simple number: an early
`if (len == 0) { return false; }` is added (after lines 785-787), and the final
`return len > 0 && s.charAt(0) != '0';` (before 791) becomes
`return len == 1 || s.charAt(0) != '0';` (after 794).

## Painting

Mapping is exact. Every disagreement survives with the human mapping, so none of it is the
matcher's.

- Minimal, Update -> None, 4 bytes: the three updated tokens `>` -> `==`, `0` -> `1`, `&&` ->
  `||` are adjacent; omnidiff paints the two spaces between them as part of one Update range,
  while the painting paints three separate tokens and leaves the spaces alone. `renderer`.
- Full, Move -> None, 38 bytes: omnidiff paints the unchanged tail `s.charAt(0) != '0';` Move on
  both sides (the line shifted by three rows and the operators in front of it changed); nothing
  in it moved. `renderer`.
- Full, None/Update -> Move, 12 bytes: the Full painting has explicit Matches of `len ` and the
  two spaces to identical text three rows lower; identical sides grade as Move, although these
  bytes only shifted with the inserted lines, like the tail the painting leaves unpainted.
  omnidiff leaves `len ` unpainted and paints the spaces Update. `grading-artifact`.

## Verdict

- Primary cause: `renderer`
- Other causes: `grading-artifact`
- Where a fix would go: `src/diff/text.rs`: Minimal gap spaces between adjacent updated tokens,
  and Full displacement painting of a row-shifted unchanged tail.
- Ground truth questionable: no.
