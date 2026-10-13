# Accuracy analysis: cpp-libreoffice-delete-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 30569 nodes, 30569 graded, N:M floor 0 visible |
| Painting, Minimal | 10 bytes wrong of 129814: renderer 10, matcher 0 |
| Painting, Full | 0 bytes wrong of 129814: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The commit deletes two helper structs, `ImplMetaReadData` and `ImplMetaWriteData` (before lines
52-71). It is a pure deletion.

## Painting

- **Bracket-only lines (Minimal, 10 bytes, `None -> Delete`, `renderer`).** The struct braces `{`
  (before lines 53, 64), the empty constructor bodies `{}` (lines 60, 69) and the closing `};`
  (lines 61, 70) stand on their own lines. Omnidiff's Minimal rendering drops them as structural
  punctuation. The Minimal painting paints them Delete with the structs. The runs are the same with
  the human mapping.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter in `src/diff/text.rs`. It would
  keep the brackets of a construct that is deleted or inserted as a whole.
- Ground truth questionable: no
