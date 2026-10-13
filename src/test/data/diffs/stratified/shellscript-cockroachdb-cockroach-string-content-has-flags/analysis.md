# Accuracy analysis: shellscript-cockroachdb-cockroach-string-content-has-flags

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 173 nodes, 173 graded, N:M floor 0 visible |
| Painting, Minimal | 914 bytes wrong of 2712: renderer 914, matcher 0 |
| Painting, Full | 1610 bytes wrong of 2712: renderer 1610, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Same commit as shellscript-cockroachdb-cockroach-another-flags-in-string-example. In the 845-
character string `BAZEL_SUPPORT_EXTRA_DOCKER_ARGS="-e ... -e DD_API_KEY"` (before line 16, after
line 19) three flags are removed: `-e COCKROACH_DEV_LICENSE`, `-e COCKROACH_RANDOM_SEED` and
`-e COCKROACH_EA_PROBABILITY`. A three-line comment is added above.

## Painting

The mapping is perfect and every run is the same with the human mapping.

- Both presets, renderer, 914 bytes in Minimal: the paintings have three Deletes of the removed
  flags (before line 16, columns 117-141, 348-372, 588-615). Omnidiff's rendering of the string
  content does not show three deletions; on the after side it paints one Update block of 416
  bytes, from `TESTS` (column 120) to the last change (column 536), and the before side from the
  first to the last deleted flag (498 bytes) disagrees too. The three small deletions are merged
  into one coarse update over the unchanged text between them. `renderer`.
- Full only, about 700 more bytes: the Full painting records the unchanged pieces of the line
  (before columns 0-116, 141-347, 372-587, 615-845 and the trailing ` \`) as Match spans. Their
  text is byte-identical on both sides, so they are scored as Move, and the rendering, which
  leaves unchanged text unpainted, disagrees on all of it. Painting the unchanged text of an
  edited line is unlike rule 3 of text_painting_findings.md (an identical tail stays unpainted)
  and unlike the Full painting of the sibling fixture
  shellscript-cockroachdb-cockroach-another-flags-in-string-example, which leaves it unpainted.
  `ground-truth` (Marko's call: whether these Match spans are meant as moves).

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth`
- Where a fix would go: string-content diff in the text renderer: keep separate small edits inside
  one long string as separate Delete/Insert runs instead of one Update span.
- Ground truth questionable: yes: the Full painting marks the unchanged parts of the edited line as
  Match (scored as Move), unlike the sibling fixture from the same commit.
