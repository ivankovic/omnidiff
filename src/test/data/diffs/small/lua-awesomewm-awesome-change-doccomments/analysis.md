# Accuracy analysis: lua-awesomewm-awesome-change-doccomments

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 21 (14 visible) of 6236 nodes, 6236 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

The commit rewrites the doc comments of the `valign` and `align` properties in
`lib/wibox/widget/textbox.lua`. It rewords the title lines, inserts a two-line paragraph and a `--`
line after each title, changes the `**top**` bullets to `` `"top"` ``, and shortens the two-line
`@tparam` descriptions to one line. The code does not change.

## Mapping

All mismatches come from the whole-file residual APTED pass (`APTED("fast_fallback")`). In Lua each
`--` line is its own top-level `comment` node, so the doc block is a run of sibling comments.

- **Which empty `--` line survives (8 visible, 4 invisible), `ambiguous`.** Before line 305 is an
  empty `--` after the title. After the change there are two empty `--` lines around the new
  paragraph: line 305 just after the title and line 308 just before `-- Valid values are:`. The
  ground truth keeps before 305 as after 308 and marks after 305 as inserted. This matches a line
  diff. omnidiff keeps before 305 as after 305 and so inserts after 308 instead. The second block
  is the same: before 333 to after 338 in the ground truth, and to after 335 in omnidiff. The nodes
  are identical and either choice inserts three lines, so both answers are defensible.
- **Reworded comments deleted instead of updated (6 visible, 3 invisible), `matcher`.** The ground
  truth updates three comments:
  - before 332 `--- Set a textbox horizontal alignment.` to after 334
    `--- The horizontal text alignment.`
  - before 315 `-- @tparam[opt="center"] string mode Where should the textbox be drawn? "top", ...`
    to after 318 `-- @tparam[opt="center"] string mode The vertical alignment`
  - before 343, the `align` twin of the 315 line, to after 348.

  omnidiff deletes the before comments (op Delete) and inserts the after comments. The `@tparam`
  pairs share only the prefix `-- @tparam[opt="..."] string mode `. The title pair shares only
  `---`, `horizontal` and `alignment.`. The matching vertical title (before
  304 to after 304) is updated correctly, so APTED's text-similarity update cost seems to be just
  over the limit for these three pairs. I did not read the cost model to confirm this. The ground
  truth is reasonable: each pair holds the same position in the same doc block.

## Verdict

- Primary cause: `matcher` (the six visible mismatches that a reader sees as delete plus insert
  instead of an update; the eight `ambiguous` mismatches are not wrong)
- Other causes: `ambiguous`
- Where a fix would go: the update cost / similarity threshold for comment leaves in
  `APTED("fast_fallback")` (src/diff/apted), or a positional pass for the comment siblings of a doc
  block
- Ground truth questionable: no
