# Accuracy analysis: java-defects4j-jsoup-79-leafnode

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 4 (4 visible) of 1231 nodes, 1231 graded, N:M floor 0 visible |
| Painting, Minimal | 39 bytes wrong of 4481: renderer 4, matcher 43 |
| Painting, Full | 43 bytes wrong of 4481: renderer 18, matcher 59 |
| Ground-truth invariant violations | none |

## The change

`LeafNode.ensureChildNodes()` stops throwing: `throw new UnsupportedOperationException("Leaf Nodes
do not have child nodes.");` (line 94) becomes `return EmptyNodes;` (line 96 after), with a new
`Collections` import and an `EmptyNodes` constant (lines 5 and 9 after).

## Mapping

- Line 94 -> 96: the ground truth deletes the `throw_statement` with its children and inserts the
  `return_statement`. omnidiff maps two leaves across them: the `;` to the `;`, and the
  `type_identifier` `UnsupportedOperationException` to the `identifier` `EmptyNodes` (different
  kinds, no shared text). 4 visible (both leaves, both sides). All four come from `APTED("qualified_name")`: APTED runs
  inside `ensureChildNodes`, which is matched by name, and pairs the leaves of its only body
  statement on each side although the statements themselves stay unpaired. The statements are of different kinds and share
  only the `;`, so the ground truth's replacement is the reasonable reading. `matcher`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began printing them.

## Painting

Matcher-caused (absent with the human mapping):

- Both presets, line 94 / 96: `UnsupportedOperationException` painted Update where the paintings
  delete it (29 bytes), `EmptyNodes` painted Update where they insert it (10 bytes). Minimal 39
  bytes. Full 43: the same runs with the space before each identifier, plus the mapped `;` painted
  Move on the before side (its row changed). `matcher`.

Renderer-caused, seen only when the HUMAN mapping is rendered (not in omnidiff's output): the wholly
deleted/inserted statement is painted as one span, so the separating space and the `;` (Minimal, 4
bytes) or the line's indentation and the `;` (Full, 18 bytes) are painted, where the paintings split
the statement into word spans and leave those bytes alone. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: `APTED("qualified_name")` (`src/diff/solve_syntax_aware_matching.rs`),
  which pairs leaves of a deleted throw_statement with an inserted return_statement
- Ground truth questionable: no
