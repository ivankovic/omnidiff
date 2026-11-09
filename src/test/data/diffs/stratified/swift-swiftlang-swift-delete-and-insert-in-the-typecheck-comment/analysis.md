# Accuracy analysis: swift-swiftlang-swift-delete-and-insert-in-the-typecheck-comment

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 64 nodes, 64 graded, N:M floor 0 visible |
| Painting, Minimal | 469 bytes wrong of 709: renderer 469, matcher 0 |
| Painting, Full | 471 bytes wrong of 709: renderer 471, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The crasher-test header comment (line 1 of
`validation-test/compiler_crashers/ConstraintSystem-applySolutionFixes-c03733.swift`) is a JSON
object. The commit deletes its leading `"aliases":[...]` entry and appends a
`"signatureNext":"ConstraintSystem::applySolution"` entry. The `"kind"`/`"signature"` part in the
middle is unchanged. The whole file has 64 nodes. The comment is a single leaf, and the mapping
pairs it correctly.

## Painting

- **Whole comment painted Update instead of Delete head / Insert tail (Minimal 469 bytes, Full
  471), `renderer`.** Both paintings paint the deleted `"aliases":[...],` (before columns 4-173)
  and the inserted `,"signatureNext":...` (after columns 130-180). They leave the 126-byte
  unchanged middle unpainted (Minimal) or mark it as a moved Match (Full). omnidiff narrows the
  comment's Update only by common prefix (`// {"`) and suffix (`"}`). Nearly the whole comment
  is therefore one Update range on each side: before bytes 5-298, after bytes 5-180. The runs are
  the same with the human mapping, so the renderer owns them. A leaf update whose text is "delete
  a head, keep a middle, add a tail" needs an inner text diff (LCS) instead of prefix/suffix
  trimming.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: update narrowing for long text leaves (comments, strings) in
  src/diff/text.rs: a character or word-level diff inside the leaf that emits Delete/Insert pieces
  and leaves (or Move-paints, in Full) an unchanged middle
- Ground truth questionable: no
