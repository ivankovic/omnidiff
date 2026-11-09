# Accuracy analysis: java-defects4j-closure-132-peepholesubstitutealternatesyntax

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 21890 nodes, 21890 graded, N:M floor 0 visible |
| Painting, Minimal | 1 bytes wrong of 112277: renderer 1, matcher 0 |
| Painting, Full | 1186 bytes wrong of 112277: renderer 1186, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-132: `PeepholeSubstituteAlternateSyntax.tryMinimizeIf` only folds an `if` with assigning
branches into a hook when the condition has no side effects or the assignment target is a name:
the condition's last operand line (before 782) gains `&&` and two continuation lines (after
782-784). The `if` body is unchanged.

## Painting

Mapping is exact. Every disagreement survives with the human mapping, so none of it is the
matcher's.

- Both presets, the condition's closing `)` on after line 784, 1 byte, None/Move -> Insert: the
  paintings include it in the inserted `(thenOp.isAssign() && ...)))` while omnidiff keeps it as
  the original `)` of the `if`, which moved from line 782 to 784 (the before `)` is unpainted in
  the paintings). `renderer`.
- Full, Move -> None, about 1167 bytes: `) {` and the entire unchanged `if` body (before lines
  782-798, after 784-800) painted Move. Nothing moved: the rows shifted by the two inserted lines
  and the condition grew in front of `) {`. `renderer`.
- Full, Insert -> None, 18 bytes: the indentation of after line 784. omnidiff paints it as part
  of the inserted operand; the Full painting paints line 783's indentation but not line 784's.
  `renderer` (the painting may simply have skipped it).

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full displacement painting in `src/diff/text.rs` (row-shifted unchanged
  block after an edit on the same line as its `{`).
- Ground truth questionable: no (the line-784 indentation and the closing `)` are minor painting
  choices).
