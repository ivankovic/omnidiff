# Accuracy analysis: rust-rust-lang-rust-remove-starting-comment

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 642 nodes, 642 graded, N:M floor 0 visible |
| Painting, Minimal | 1 bytes wrong of 1980: renderer 0, matcher 1 |
| Painting, Full | 0 bytes wrong of 1980: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

From rust-lang/rust (`src/tools/miri/tests/pass/packed-struct-dyn-trait.rs`, commit a3e469ea):
the first line `// run-pass` is removed.

## Painting

The mapping matches the ground truth on every node and Full matches the single painting (`Only
one solution`) exactly. Minimal differs by 1 byte: `None -> Delete`, the space after `//` on
line 1. The painting deletes the whole line. The human file marks the comment
`delete_with_children`, and rendered that way the line is painted whole; omnidiff deletes the
same nodes (the `line_comment` and its `//`), and the renderer leaves the space between `//` and
the comment text unpainted. No graded node differs, so the census attributes the byte to the
matcher, but it is the renderer's gap after `//`, decided by how the deletion is expressed.
`renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs`: paint the text after `//` of a deleted or inserted
  line comment whole, whether or not the operation is "with children"
- Ground truth questionable: no
