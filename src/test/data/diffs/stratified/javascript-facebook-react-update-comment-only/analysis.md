# Accuracy analysis: javascript-facebook-react-update-comment-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 70 nodes, 70 graded, N:M floor 0 visible |
| Painting, Minimal | 1 bytes wrong of 311: renderer 1, matcher 0 |
| Painting, Full | 0 bytes wrong of 311: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

React compiler test fixture: the pragma comment on line 1 gains ` @outputMode:"lint"`. Nothing else
changes.

## Painting

One run, 1 byte, Minimal only, None -> Insert: the space at after line 1 col 45, in front of
`@outputMode:"lint"`. The single painting (`Only one solution`) includes that leading space in the
Insert; omnidiff's Minimal rendering trims leading and trailing whitespace off every range
(`diff::text::ranges_for_options`), Full does not and agrees exactly. The run survives with the
human mapping. It exists only because a Minimal rendering is held to a painting that keeps the
edge space; which of the two spaces around the insertion is "new" is not decidable anyway.
`grading-artifact`.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: none
- Where a fix would go: none
- Ground truth questionable: no (the painted leading space goes against the hand-painting rule
  that range-edge whitespace is not painted, but for an insertion inside a comment it is a
  defensible choice)
