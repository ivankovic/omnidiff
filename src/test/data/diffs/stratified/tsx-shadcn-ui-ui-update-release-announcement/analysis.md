# Accuracy analysis: tsx-shadcn-ui-ui-update-release-announcement

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 214 nodes, 214 graded, N:M floor 0 visible |
| Painting, Minimal | 93 bytes wrong of 849: renderer 93, matcher 0 |
| Painting, Full | 129 bytes wrong of 849: renderer 129, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The announcement text `Now available: shadcn CLI 3.0 and MCP Server` (before line 10) is replaced
by a new `<span ... title="New" />` (after line 10) and the text
`New Components: Field, Input Group, Item and more` (after line 11); `<ArrowRightIcon />` stays
at the end of the text line.

## Painting

One painting (`Minimal`), scored under both presets. All runs are identical with the human
mapping, so none is the matcher's.

- 93 bytes, both presets, the two texts (before line 10 cols 8-51, after line 11 cols 8-56),
  Update -> Delete / Insert (`renderer`). The tree (human and omnidiff alike) maps the `jsx_text`
  as an Update; the painting paints a full Delete and Insert, since the texts share nothing
  meaningful. omnidiff renders the Update as a character-narrowed Update (the run's first byte,
  the shared `N`, is unpainted). The renderer has no rule to show an Update whose two sides are
  essentially unrelated text as delete + insert.
- 36 bytes, Full only, `<ArrowRightIcon />` on both sides, Move -> None (`grading-artifact`):
  the icon moves from line 10 to line 11 because the span was inserted above, and Full paints
  it Move; the single Minimal painting leaves it unpainted.

## Verdict

- Primary cause: `renderer`
- Other causes: `grading-artifact`
- Where a fix would go: src/diff/text.rs, rendering of an Update whose texts have almost no
  common content (paint as Delete + Insert)
- Ground truth questionable: no (the tree says Update and the painting says replace; both are
  readable, the renderer cannot bridge them)
