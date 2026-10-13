# Accuracy analysis: java-defects4j-jacksondatabind-20-objectnode

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 32 (22 visible) of 7564 nodes, 7564 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 48888: renderer 116, matcher 116 |
| Painting, Full | 0 bytes wrong of 48888: renderer 121, matcher 121 |
| Ground-truth invariant violations | 7 |

## The change

Adds `import com.fasterxml.jackson.annotation.JsonIgnore;` (line 4 after) and a `@JsonIgnore //
work-around for [databind#815]` annotation (line 325 after) on a method.

## Mapping

All 32 mismatches (22 visible, 10 invisible) are one group on the imports. The ground truth maps the
before `import com.fasterxml.jackson.annotation.JsonAutoDetect;` (line 3) and all its descendants to
the after line 4, `import com.fasterxml.jackson.annotation.JsonIgnore;` (`JsonAutoDetect` ->
`JsonIgnore` as an update), and marks the unchanged after line 3, the same `JsonAutoDetect` import,
as inserted with children. omnidiff (`IdenticalHash` on the `import_declaration`,
`IdenticalHashOfAncestor` on its descendants, on both sides) maps the identical import to its
identical twin on line 3 and inserts the new `JsonIgnore` import.

omnidiff's mapping is the natural one, and it is the one both paintings describe: they insert the
whole `JsonIgnore` line. The ground truth's own invariant check reports 7 violations here: both
paintings leave unpainted the after line-3 leaves the mapping calls inserted, paint inserted the
line-4 leaves the mapping pairs with identical ones, and the `JsonAutoDetect` -> `JsonIgnore`
pairing is not painted as the mapping says. `ground-truth`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Painting

omnidiff's rendering matches both paintings exactly (0 bytes). Only the HUMAN mapping, rendered,
disagrees with them (116 / 121 renderer bytes): it paints the unchanged line-3 import inserted,
`AutoDetect` updated, and leaves the new `JsonIgnore` import unpainted. That is the same
ground-truth slip seen through the renderer; no reader of omnidiff's output sees it.

## Verdict

- Primary cause: `ground-truth`
- Other causes: none
- Where a fix would go: ground truth (Marko's call); nothing in the engine
- Ground truth questionable: yes: human mapping pairs import JsonAutoDetect (line 3) with the new
  import JsonIgnore (line 4) and inserts the identical line 3; 7 invariant violations; both
  paintings insert the JsonIgnore line
