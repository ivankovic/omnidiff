# Accuracy analysis: java-defects4j-closure-80-nodeutil

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (0 visible) of 36520 nodes, 36520 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 175922: renderer 0, matcher 0 |
| Painting, Full | 12 bytes wrong of 175922: renderer 12, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Adds `case Token.DELPROP:` to two switches (line 1262 and line 2911 after), each after a `// delete
operator returns a boolean.` comment, so `delete` is treated as returning a boolean.

## Mapping

- Line 1261, the `switch_block_statement_group` `case Token.NOT: // delete ... return true;` (1
  invisible): the ground truth maps it to the after group, which gains the new label. omnidiff
  (`APTED("large_flat_subtree")`) leaves the group container unmapped (Delete) while its children
  still map, so nothing is painted differently. The same edit at line 2910 is mapped correctly.
  `matcher`, invisible.

## Painting

Minimal matches the single `Minimal` painting exactly. Full, `Insert -> None`, the 6-byte
indentation of each inserted `case Token.DELPROP:` line (lines 1262 and 2911 after, 12 bytes): Full
paints indentation, the Minimal painting it is held to does not. Same with the human mapping.
`grading-artifact`.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: `matcher`
- Where a fix would go: none for what a reader sees; the invisible container miss is in the APTED
  fallback (large_flat_subtree)
- Ground truth questionable: no
