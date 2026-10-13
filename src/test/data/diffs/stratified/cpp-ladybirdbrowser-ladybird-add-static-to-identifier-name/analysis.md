# Accuracy analysis: cpp-ladybirdbrowser-ladybird-add-static-to-identifier-name

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 23300 nodes, 23300 graded, N:M floor 0 visible |
| Painting, Minimal | 12 bytes wrong of 125250: renderer 12, matcher 0 |
| Painting, Full | 0 bytes wrong of 125250: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Two `static Bytecode::PropertyLookupCache cache;` declarations (lines 1566 and 1570) change their
type to `Bytecode::StaticPropertyLookupCache`.

## Painting

- **Inserted word labelled Update (Minimal, 12 bytes, `Update -> Insert`, `renderer`).** Omnidiff
  narrows the identifier update to the differing word `Static` on after lines 1566 and 1570. The
  range is right, but the label is Update. The Minimal painting labels a word added inside an
  identifier Insert, with nothing on the before side. The run is the same with the human mapping.
  Full agrees with omnidiff.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the word-level narrowing of identifier Updates in `src/diff/text.rs`. A pure
  word insertion would be labelled Insert in Minimal.
- Ground truth questionable: no
