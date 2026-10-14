# Accuracy analysis: java-defects4j-mockito-19-typebasedcandidatefilter

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 443 nodes, 443 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 1872: renderer 0, matcher 0 |
| Painting, Full | 26 bytes wrong of 1872: renderer 26, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Mockito-19. `TypeBasedCandidateFilter.filterCandidate` gets a `List<Field> fields`
parameter (line 20) and passes it to the next filter (line 28). One painting is on file (`Only one
solution`), so it scores both presets.

## Painting

Full only, 26 bytes, renderer-owned (the same with the human mapping). On line 28, `return
next.filterCandidate(mockTypeMatches, field, fieldInstance);` gains `fields, ` before the last
argument, and Full paints `fieldInstance` Move on both sides. The painting leaves it unpainted. This
is not only the Minimal painting being held to Full: on line 20 the parameter `Object
fieldInstance`, shifted the same way, is left unpainted by Full. So Full is inconsistent on its own
terms. `renderer`.

The two rows differ in `node_untouched_on_its_row` (`src/diff/text.rs`). On line 28 the inserted
`fields` begins with `field`, so the common prefix of the two rows runs into `fieldInstance`. The
common suffix is clamped to the rest (`Instance);`), and the node is in neither. Minimal still
exempts it through its same-row fallback (`read_rows_carrying_several_edits`), and Full has no such
fallback. On line 20 the prefix stops at `Object`/`List<`, so the parameter sits wholly in the
unchanged suffix.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` `node_untouched_on_its_row`: the prefix/suffix
  computation when an insertion shares leading characters with the following node
- Ground truth questionable: no
