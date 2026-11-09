# Accuracy analysis: cpp-cgdb-cgdb-struct-to-pointer

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 110 (73 visible) of 11436 nodes, 11436 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`update_file_position` now takes `const tgdb_file_position &tfp` instead of
`struct tgdb_response *response`. The local `tfp` pointer goes away, and every `tfp->x` becomes
`tfp.x`. The `TGDB_UPDATE_FILE_POSITION` case leaves `command_response`, and a new callback
`update_file_pos` is added. description.md: "Interesting case of which nodes of different kinds
should be allowed to map to each other". The ground truth maps `struct` to `const`, `*` to `&`
and `->` to `.`.

## Mapping

All 110 mismatches are on the before side, in two functions: 88 in `update_file_position`
(lines 501-548) and 22 in `command_response` (lines 669-681).

- **Wrong twin for `struct tgdb_response` (2 visible, 1 invisible, `matcher`).**
  `struct tgdb_response` occurs three times in before (lines 373, 501, 669) and twice in after
  (lines 373, 670). Phase 1 (`IdenticalHash`) matched before line 501's copy, the parameter of
  `update_file_position`, to after line 670, the parameter of `command_response`. Its tokens
  `struct` and `tgdb_response` followed (`IdenticalHashOfAncestor`). The ground truth removes this
  `struct_specifier`, and it maps `struct` -> `const` and `tgdb_response` -> `tgdb_file_position`.
  The correct twin for after line 670 is before line 669.
- **Function frames deleted (71 visible, 36 invisible, `matcher`, `APTED("qualified_name")`).**
  The qualified-name pass pairs the two functions by name, but APTED deletes their frames: the
  `function_definition` nodes, `static void update_file_position(`, `)`, and the `if` header at
  line 532 with its block. Other deleted nodes are the statements whose `tfp->` changed to `tfp.`
  (lines 529, 540, 543-547) and the `->` tokens, which the ground truth maps to `.`. In
  `command_response` the deleted nodes are `static void command_response(...)`, `{`, `switch` and
  the braces (lines 669-672, 680, 681). The unchanged lines in the bodies are not on the list.
  Most likely cause: after the hash match above, a descendant of before line 501 is mapped into
  after `command_response`. Thus pairing before 669 with after 670, or before 501 with after
  505, is no longer ancestry-consistent, and APTED must delete one side of each frame. This is an
  inference from the evidence. The engine was not run.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the phase-1 `IdenticalHash` twin choice for small repeated subtrees, which
  should prefer the twin whose enclosing named function matches. It runs before the
  `qualified_name` APTED call in `src/diff/solve_syntax_aware_matching.rs`.
- Ground truth questionable: no. The cross-kind pairs (`struct` -> `const`, `*` -> `&`,
  `->` -> `.`) are deliberate, as description.md says.
