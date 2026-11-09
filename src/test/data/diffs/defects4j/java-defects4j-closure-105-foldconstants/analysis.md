# Accuracy analysis: java-defects4j-closure-105-foldconstants

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 24 (16 visible) of 29325 nodes, 29325 graded, N:M floor 0 visible |
| Painting, Minimal | 130 bytes wrong of 126502: renderer 89, matcher 60 |
| Painting, Full | 396 bytes wrong of 126502: renderer 423, matcher 128 |
| Ground-truth invariant violations | 1 |

## The change

Closure-105: `FoldConstants.tryFoldArrayJoin` creates the `StringBuilder` lazily. The declaration
`StringBuilder sb = new StringBuilder();` (line 1477) becomes `= null;`, the first string branch
gains `if (sb == null) { sb = new StringBuilder(); } else { sb.append(joinString); }` (after
lines 1483-1487), the reset `sb = new StringBuilder();` (before 1492) becomes `sb = null;`, and the
three `sb.length() > 0` tests become null tests.

## Mapping

- Two identical `new StringBuilder()` sources, 8 visible + 4 invisible. Three `new StringBuilder()`
  expressions exist: before lines 1477 and 1492, after line 1484. Both before ones are replaced by
  `null` in place. The ground truth (tree and painting) moves the reset's (before 1492) expression
  to after 1484 and deletes the declaration's. omnidiff moves the declaration's (before 1477,
  `MovedSubtree`, 4 visible + 2 invisible: the move pass pairs a fully deleted subtree with a
  byte-identical inserted one, and both before copies qualify) and deletes the reset's (op Delete,
  `APTED("large_flat_subtree")`, 4 visible + 2 invisible). Either source reads as "the builder is
  created here now"; nothing in the text prefers one. `ambiguous`.
- The two later conditions, 8 visible + 4 invisible: before lines 1488 and 1500
  `sb.length() > 0` -> after 1490 and 1502 `sb != null`. The ground truth deletes and inserts the
  whole `binary_expression`; omnidiff matches the two binary expressions (`sb` -> `sb`, `>` ->
  `!=`; `APTED("large_flat_subtree")`). For the first, identically shaped condition (line 1483,
  `sb.length() > 0` -> `sb == null`) the ground truth's tree mapping does match the
  `binary_expression`, and invariants.txt flags it (rule 18: `0` deleted and `null` inserted
  under matched parents). The ground truth treats the same edit two ways; omnidiff treats all
  three the same way as the first. `ground-truth`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper
began printing them.

## Painting

- Matcher, Minimal 60 bytes / Full 128: the twin choice paints before 1477 `new StringBuilder()`
  as kept (None in Minimal, Move in Full) where the painting deletes it, and before 1492 as
  Delete where the painting has Move (19-20 bytes each); matching the conditions leaves `sb` and
  ` > ` / `sb != ` unpainted where the painting paints the whole conditions Delete/Insert (about
  10 bytes per side). `ambiguous` / `ground-truth` as above.
- Renderer, Minimal 89 bytes (all survive with the human mapping):
  - Moves into another block not painted, None -> Move, 83 bytes: `sb.append(joinString);`
    moves from the `if` body to the new `else` body (before 1484, after 1486, 22 bytes per side),
    and `new StringBuilder()` from before 1492 to after 1484 (19 + 20). omnidiff paints them
    unpainted. `renderer`.
  - Brace attribution in `} else {`, 6 bytes: omnidiff paints the after line-1483 `{` and the
    `} ` of line 1485 Insert and keeps the final `}`; the painting keeps those and inserts
    ` {`/`else {` and the line-1487 `}`. `renderer`.
- Renderer, Full 423 bytes (human mapping): beyond the above, Full paints Move over unchanged
  text that only shifted rows: the `{`, `}`, indentation and `) {` around the edited conditions,
  and the whole unchanged body of the final `if` (before lines 1501-1504, after 1503-1506, about
  130 bytes per side), plus `;` on after line 1494. The painting leaves all of it unpainted.
  `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `ambiguous`, `ground-truth`
- Where a fix would go: `src/diff/text.rs`: Full's Move painting of row-shifted unchanged text,
  and Minimal's painting of identical subtrees moved into a different block.
- Ground truth questionable: yes: invariant 18 on line 1483; the first `sb.length() > 0` is
  matched as a binary expression while the other two identical edits (lines 1488, 1500) are
  deleted and inserted wholesale.
