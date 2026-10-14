# Accuracy analysis: xml-mozilla-firefox-firefox-add-four-attributes

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 240 nodes, 240 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 1883: renderer 2, matcher 0 |
| Painting, Full | 2 bytes wrong of 1883: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Firefox for Android, app-links `values-an/strings.xml`: the root `<resources>` gains two namespace
declarations (line 9) and one `<string>` gains `tools:ignore="UnusedResources"
moz:removedIn="150"` (line 13).

## Painting

There are no mapping mismatches; the runs are identical with the human mapping, so all of it is the
renderer's. 2 bytes per preset, single spaces around the inserted attributes on after lines 9 and
13:

- Minimal, Insert -> None, the space between the two inserted attributes (line 9 col 58, line 13
  col 92). The Minimal painting inserts each attribute separately and leaves the gap; omnidiff
  joins adjacent insertions into one range including the gap. `renderer`.
- Full, None -> Insert, the space in front of the first inserted attribute (line 9 col 11, line
  13 col 61). The Full painting inserts it; omnidiff starts its range at the attribute. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` gap and edge whitespace of adjacent inserted attributes
- Ground truth questionable: no
