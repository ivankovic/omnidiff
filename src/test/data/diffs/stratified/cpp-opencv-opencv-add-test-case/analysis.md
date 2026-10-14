# Accuracy analysis: cpp-opencv-opencv-add-test-case

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 6850 nodes, 6850 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 24062: renderer 4, matcher 0 |
| Painting, Full | 0 bytes wrong of 24062: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The commit inserts a new test `TEST_P(Test_TFLite, minimum) { ... }` (after lines 314-341). It is a
pure insertion.

## Painting

- **Brace lines of the inserted test (Minimal, 4 bytes, `None -> Insert`, `renderer`).** The `{` and
  `}` of the test body (after lines 315, 340) and of its `if` block (after lines 334, 337) stand on
  their own lines. Omnidiff's Minimal rendering drops them as structural punctuation. The Minimal
  painting inserts them with the test. The runs are the same with the human mapping.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter in `src/diff/text.rs`. It would
  keep the brackets of a construct that is inserted or deleted as a whole.
- Ground truth questionable: no
