# Accuracy analysis: java-defects4j-mockito-19-namebasedcandidatefilter

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 740 nodes, 740 graded, N:M floor 0 visible |
| Painting, Minimal | 26 bytes wrong of 3358: renderer 26, matcher 0 |
| Painting, Full | 31 bytes wrong of 3358: renderer 31, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Mockito-19. `NameBasedCandidateFilter.filterCandidate` gets a `List<Field> fields`
parameter. It passes it on in both delegating calls, and adds an `else if (mocks.size() == 1)`
branch that checks the other fields for a name match. Indentation is tabs.

## Painting

All runs are renderer-owned: rendering the human mapping gives the same bytes.

- **`Move -> None`, Minimal 26 bytes, Full 30.** In the final `return next.filterCandidate(mocks,
  field, fieldInstance);` (line 43 before, line 59 after), the inserted `fields, ` pushes
  `fieldInstance` (and in Full the `);`) right. It is painted Move on both sides. The painting
  leaves this identical line tail unpainted (rule 3 of `text_painting_findings.md`). The row also
  moved down by 16 lines, because of the inserted `else if` branch. The same tail on the delegating
  call at line 32, which sits on its own continuation line, is not affected. The likely mechanism is
  in `node_untouched_on_its_row` (`src/diff/text.rs`). The inserted `fields` begins with the same
  letters as `fieldInstance`, so the rows' common prefix runs through `field, field`. The common
  suffix is then clamped to what is left after that prefix (`Instance);`). The node therefore lies
  in neither the unchanged prefix nor the unchanged suffix. The same-row fallback cannot help,
  because the row index changed (43 -> 59). `renderer`.
- **Full, 1 byte (`None -> Insert`).** The Full painting inserts the space between `}` and the new
  `else if` on line 42. The rendering leaves it. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` `node_untouched_on_its_row`: the prefix/suffix
  computation, when an insertion shares leading characters with the node that follows it
- Ground truth questionable: no
