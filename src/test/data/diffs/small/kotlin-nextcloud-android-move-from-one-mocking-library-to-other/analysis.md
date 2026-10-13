# Accuracy analysis: kotlin-nextcloud-android-move-from-one-mocking-library-to-other

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 24 (12 visible) of 1070 nodes, 1070 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

A test moves from Mockito's `@Captor ArgumentCaptor` to mockito-kotlin's `argumentCaptor`. The
class field `@Captor private lateinit var filesServiceCallbackCaptor:
ArgumentCaptor<FilesServiceCallback<OCFile>>` (before line 33) is removed, and each of the two
tests gets its own local `val captor = argumentCaptor<FilesServiceCallback<OCFile>>()` (after
lines 42 and 52). Imports and uses are adjusted.

## Mapping

All 24 mismatches are one subtree: the type arguments `<FilesServiceCallback<OCFile>>` (before
line 33, columns 76-106) and the same text in the first new local (after line 42, columns
36-66), 6 visible leaves (`<`, `FilesServiceCallback`, `<`, `OCFile`, `>`, `>`) and 6 invisible
type nodes on each side.

- The ground truth deletes the whole field declaration and inserts both locals in full.
- Omnidiff maps the identical type-argument subtree from the field to the first local, as a move
  (`MovedSubtree`, all 24: a fully deleted subtree paired with a byte-identical inserted one).
- Both are defensible. The type argument is the same text and the same role (the captor's type),
  and the field did become a local, which argues for a move. But the one field became two locals,
  a 1:2 split, and the ground truth does not prefer either copy, so it treats both as new.
  Omnidiff picks the first copy. `ambiguous`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper
began printing them.

## Verdict

- Primary cause: `ambiguous`
- Other causes: none
- Where a fix would go: none: ambiguous (a matcher that refuses moves into one of several
  identical targets would match the ground truth here).
- Ground truth questionable: no
