# Accuracy analysis: cpp-microsoft-terminal-insert-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1253 nodes, 1253 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 6827: renderer 4, matcher 0 |
| Painting, Full | 0 bytes wrong of 6827: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The commit inserts a comment and an `if (source->_FontAxes) { ... for (...) { ... } ... }` block
that deep-copies the font axes (after lines 33-44). It is a pure insertion.

## Painting

- **Brace lines of the inserted block (Minimal, 4 bytes, `None -> Insert`, `renderer`).** The `{`
  and `}` of the `if` and `for` bodies (after lines 36, 39, 41, 43) stand on their own lines.
  Omnidiff's Minimal rendering drops them as structural punctuation. The Minimal painting inserts
  them with the block. The runs are the same with the human mapping.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter in `src/diff/text.rs`. It would
  keep the brackets of a construct that is inserted or deleted as a whole.
- Ground truth questionable: no
