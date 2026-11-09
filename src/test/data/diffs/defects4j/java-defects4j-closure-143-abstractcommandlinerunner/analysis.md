# Accuracy analysis: java-defects4j-closure-143-abstractcommandlinerunner

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 14526 nodes, 14526 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 78321: renderer 0, matcher 0 |
| Painting, Full | 812 bytes wrong of 78321: renderer 812, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-143: `AbstractCommandLineRunner.createDefineReplacements` also accepts double-quoted
define values: the condition's closing line `)) {` (before 871) gains a new operand
`|| (defValue.charAt(0) == '\"' && defValue.charAt(defValue.length() - 1) == '\"'))) {` (after
871-872). The block below is unchanged.

## Painting

Mapping is exact and Minimal matches its painting. All 812 Full bytes survive with the human
mapping, so they are renderer-owned.

- Full, Move -> None, about 780 bytes: the unchanged `if` body after the edited line (the two
  comment lines, `String maybeStringVal = ...`, the inner `if`, `continue;` and two `}`; before
  872-880, after 873-881) painted Move on both sides. Nothing moved; the rows shifted by the one
  added line. `renderer`.
- Full, the edited line's indentation and `)) {`, 32 bytes: the Full painting matches the old
  line's 12-byte indentation to the indentation in front of the new second operand line (Move)
  and leaves `)) {` unpainted; omnidiff paints that indentation Insert and the relocated `)) {`
  Move, and leaves the old line's indentation unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full displacement painting in `src/diff/text.rs` (row-shifted unchanged
  block after an edit on the line holding its `{`).
- Ground truth questionable: no.
