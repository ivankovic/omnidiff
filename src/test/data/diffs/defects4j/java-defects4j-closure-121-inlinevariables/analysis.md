# Accuracy analysis: java-defects4j-closure-121-inlinevariables

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 9328 nodes, 9328 graded, N:M floor 0 visible |
| Painting, Minimal | 5 bytes wrong of 52503: renderer 5, matcher 0 |
| Painting, Full | 413 bytes wrong of 52503: renderer 413, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-121: `InlineVariables` inlines a variable assigned once only if it is also a declared
constant or assigned in its declaring scope: the `if` condition ending on line 304
(`referenceInfo.isAssignedOnceInLifetime()) {`) gains `&&` and a two-line disjunction after the
existing comment (after lines 308-309), so the condition's `)` and the block's `{` now end line
309.

## Painting

Mapping is exact. Every disagreement survives with the human mapping (renderer 5 / 413 bytes =
real), so none of it is the matcher's.

- Both presets, the block's braces, None -> Move, 4 bytes in Minimal, 14 in Full (with the `}`
  line's indentation): the `{` relocates from the end of line 304 to the end of line 309 and the
  paintings paint it, and its partner `}` (line 326 -> 328), Move. omnidiff leaves the `{` and
  `}` unpainted in Minimal and the `}` line in Full. `renderer`.
- Both presets, the condition's closing `)`, 1-2 bytes: omnidiff keeps it (unpainted in Minimal,
  Move in Full on both sides); the paintings include it in the inserted
  `referenceInfo.isOnlyAssignmentSameScopeAsDeclaration()))` on line 309 and leave the before
  `)` on line 304 unpainted, so the before `)` has no counterpart in the painting. Full also
  disagrees on one inserted space on line 304. `renderer`, with a questionable painting detail.
- Full, None -> Move, 396 bytes: the three comment lines 305-307. The Full painting has an
  explicit `Match` of the unchanged comment to itself at the same rows and columns; identical
  sides grade as Move, while omnidiff leaves unchanged text unpainted. `grading-artifact`.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: `renderer`
- Where a fix would go: none in the engine for the 396 bytes (grader treatment of an in-place
  identical Match); `src/diff/text.rs` for relocated brackets.
- Ground truth questionable: yes, mildly: the before `)` on line 304 is unpainted while its after
  counterpart on line 309 is painted Insert.
