# Accuracy analysis: rust-quietvoid-dovi_tool-change-to-templated-call

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 6 (6 visible) of 1689 nodes, 1689 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

From quietvoid/dovi_tool (`dolby_vision/src/rpu/extension_metadata/blocks/level1.rs`, commit
395c4594): a bitstream API change. `reader.get_n(12)?` becomes `reader.read::<12, u16>()?` (lines
37-39) and `writer.write_n(&self.min_pq, 12)?` becomes `writer.write::<12, u16>(self.min_pq)?`
(lines 46-48): the bit width moves from a call argument into a const generic argument.

## Mapping

Six visible mismatches, one cause, reason `APTED("qualified_name")`: each `12` (before lines
37-39, column 34, and 46-48, column 38). The ground truth maps every old argument `12` to the
`12` in the new turbofish of the same line. omnidiff deletes the old ones (and inserts the new
ones). The tree edit distance keeps the old and new `arguments` nodes paired, and with that
pairing an argument cannot also map into the callee's `type_arguments` without breaking ancestry;
`12` occurs six times on each side, so no uniqueness-based move pass reclaims it either. A move
not detected. `matcher`.

## Painting

None: the fixture has no painting.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: a move pass after `APTED("qualified_name")` for a leaf that moves from a
  call's arguments into its callee's generic arguments on the same line (pairing repeated twins
  by line, since the literal is not unique)
- Ground truth questionable: no
