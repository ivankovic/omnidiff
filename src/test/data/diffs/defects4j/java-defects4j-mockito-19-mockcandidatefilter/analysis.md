# Accuracy analysis: java-defects4j-mockito-19-mockcandidatefilter

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 169 nodes, 169 graded, N:M floor 0 visible |
| Painting, Minimal | 31 bytes wrong of 916: renderer 31, matcher 0 |
| Painting, Full | 3 bytes wrong of 916: renderer 3, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Mockito-19. In the `MockCandidateFilter` interface, the last parameter `Object
fieldInstance` of `filterCandidate` becomes `List<Field> fields, Object instance`, and
`java.util.List` is imported.

## Painting

All runs are renderer-owned: rendering the human mapping gives the same bytes.

- **Minimal, 31 bytes.** On line 15 before and line 16 after, the parameter type `Object` was pushed
  right by the inserted `List<Field> fields,`. Minimal paints it Move, and the Minimal painting
  leaves it unpainted. The Full painting does paint `Object ` Move, and there omnidiff agrees. The
  rename `fieldInstance` -> `instance` is painted by the Minimal painting as Delete `field` plus
  Update `Instance` -> `instance`. omnidiff's intra-node Update split instead paints `field` (and
  the `I`/`i`) as Update and leaves `nstance` unpainted (6 + 5 + 7 + 6 + 7 bytes). `renderer`.
- **Full, 3 bytes.** The Full painting's `Object ` Move includes the trailing space, and it inserts
  the space after `fields,`. The Full rendering leaves both spaces unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs`: the intra-identifier Update split for a rename that
  drops a prefix (Minimal), and Minimal Move painting of a type pushed right by an inserted
  parameter
- Ground truth questionable: no
