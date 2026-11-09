# Accuracy analysis: java-defects4j-jacksondatabind-15-stdserializer

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 3157 nodes, 3157 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 21911: renderer 0, matcher 0 |
| Painting, Full | 1 bytes wrong of 21911: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Line 262: `if (existingSerializer == null)` gains the conjunct `&&
!delegateType.hasRawClass(Object.class)`, so a delegate of type `Object` no longer gets a serializer
looked up.

## Painting

Minimal matches exactly. Full, `None -> Insert`, line 262 after side, 1 byte: the space before the
inserted conjunct, which the Full painting includes and omnidiff's Full rendering does not. Same
with the human mapping. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs Full gap handling: leading separator whitespace of an
  inserted operand
- Ground truth questionable: no
