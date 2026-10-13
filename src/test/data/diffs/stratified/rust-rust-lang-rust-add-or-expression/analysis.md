# Accuracy analysis: rust-rust-lang-rust-add-or-expression

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 4188 nodes, 4188 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 18141: renderer 0, matcher 0 |
| Painting, Full | 1 bytes wrong of 18141: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

From rust-lang/rust (`src/librustdoc/passes/calculate_doc_coverage.rs`, commit 6fcd0115): one
match arm pattern gains an alternative, `clean::StrippedItem(..) =>` -> `clean::StrippedItem(..)
| clean::PlaceholderImplItem =>` (line 202).

## Painting

The mapping matches the ground truth on every node and Minimal matches its painting exactly.
Full differs by 1 byte, identical with the human mapping: `None -> Insert`, the space in front of
the inserted `| clean::PlaceholderImplItem` on after line 202. The Full painting starts the
`Insert` at that space (column 35), omnidiff at the `|` (column 36). `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs`, Full: include the separating space in front of an
  inserted `or_pattern` alternative (1 byte; small enough to leave)
- Ground truth questionable: no
