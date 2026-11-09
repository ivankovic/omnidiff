# Accuracy analysis: cpp-nzbgetcom-nzbget-add-if

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 16170 nodes, 16170 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 55800: renderer 2, matcher 0 |
| Painting, Full | 0 bytes wrong of 55800: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The commit inserts
`if (!nzbFile.GetPassword().empty()) { ... SetParameter("*Unpack:Password", ...); }` (after lines
678-683). It is a pure insertion.

## Painting

- **Braces of the inserted if (Minimal, 2 bytes, `None -> Insert`, `renderer`).** The `{` (after
  line 680) and `}` (after line 682) stand on their own lines. Omnidiff's Minimal rendering drops
  them as structural punctuation. The Minimal painting inserts them with the `if`. The runs are the
  same with the human mapping.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter in `src/diff/text.rs`. It would
  keep the brackets of a construct that is inserted or deleted as a whole.
- Ground truth questionable: no
