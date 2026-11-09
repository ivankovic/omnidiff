# Accuracy analysis: java-defects4j-mockito-19-finalmockcandidatefilter

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 645 nodes, 645 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 3519: renderer 0, matcher 0 |
| Painting, Full | 1618 bytes wrong of 3519: renderer 1618, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Mockito-19 (README.md): the fix adds a `List<Field> fields` parameter before
`final Object fieldInstance` in `filterCandidate` (line 24) and adds `import java.util.List;`
(line 13). Nothing else changes.

## Painting

The single painting (`Only one solution`) is scored under both presets. Minimal matches it
exactly. The Full runs are identical with the human mapping, so all of them belong to the renderer.

- **Move -> None, 1618 bytes: before lines 23-47, after lines 24-48.** Under Full, omnidiff paints
  `final Object fieldInstance) {` (29 bytes per side) and every line of the method body down to the
  method's closing `    }` (780 bytes per side) as Move. The painting leaves all of it unpainted.
  The parameter and the body block start on the edited line and were pushed right by the inserted
  `List<Field> fields, `. In `src/diff/text.rs` `identical_or_move`, the two exemptions for such a
  displacement are gated, under `FULL` (`paint_displaced_moves: true`), on the node keeping its row
  index: `s.start_row == d.start_row` for `shifted_within_its_own_line` and
  `shifted_by_an_edit_beside_it`. The disjunct that accepts an equal row shift
  (`displaced_beside_an_edit_on_its_first_row`) is enabled only when `paint_displaced_moves` is off.
  The import inserted at line 13 shifts every later row by one, so the gates fail.
  `column_shift_is_meaningful` then holds, and the 24-line block paints Move because its first line
  moved sideways. Minimal accepts the equal row shift and paints nothing. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` `identical_or_move`: the FULL-only row-index gating of
  the displacement exemptions. A whole multi-line block whose first line slid sideways paints Move
  only when an unrelated edit above also shifts rows.
- Ground truth questionable: no
