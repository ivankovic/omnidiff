# Accuracy analysis: lua-awesomewm-awesome-insert-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 18 (12 visible) of 1018 nodes, 1018 graded, N:M floor 0 visible |
| Painting, Minimal | 82 bytes wrong of 2587: renderer 0, matcher 82 |
| Painting, Full | 82 bytes wrong of 2587: renderer 0, matcher 82 |
| Ground-truth invariant violations | none |

## The change

The commit inserts four lines into the awful.placement example
`tests/examples/awful/placement/skip_fullscreen.lua`. They are two `-- using ...` comments (after
lines 7 and 16) and two `print("...") --DOC_HIDE` lines (after lines 10 and 19). Nothing else
changes. There is one painting (`Only one solution`), used for both presets.

## Mapping

- **Trailing `--DOC_HIDE` comments paired with the wrong twin (12 visible, 6 invisible),
  `matcher`.** In tree-sitter-lua a trailing `--DOC_HIDE` is a top-level `comment` sibling after
  its statement. The two blocks fail in the same way:
  - The ground truth keeps before line 8's comment (`c.fullscreen = true --DOC_HIDE`) as after
    line 9, and before line 9's comment (the `print("Before:" ...)` line) as after line 11. It
    marks the comment of the inserted `print("no_offscreen:") --DOC_HIDE` (after line 10) as
    inserted.
  - omnidiff pairs before line 8's comment with after line 10's comment (reason `LeadingSibling`
    for the `comment`, `IdenticalHashOfAncestor` for its `--` and content; the after-side
    "descendant" lines print the same reasons). 8 visible, 4 invisible.
    `solve_leading_siblings` sees a comment just before the matched `print("Before:" ...)`
    statement on both sides and pairs the two. But these comments trail the statement before them
    on the same line, so they do not lead the next statement.
  - `APTED("fast_fallback")` then pairs the remaining before line 9 comment with after line 9.
    After line 11's comment is left inserted. 4 visible, 2 invisible.

  The second block (before 15/16, after 18/19/20) is the same.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Painting

All 82 bytes are the matcher's: with the human mapping the renderer agrees with the painting
exactly. The runs are the same in both presets. The Move runs are at slightly different rows,
because Minimal and Full attribute a crossed pair to different sides.

- **Move -> None, 40 bytes, `matcher`.** The crossed `--DOC_HIDE` pairs paint as moves:
  - Minimal: before lines 9 and 16, after lines 9 and 18.
  - Full: before lines 8, 15 and 16, after line 18.
- **None -> Insert, 22 bytes, `matcher`.** ` --DOC_HIDE` of the inserted print lines (after lines
  10 and 19) is left unpainted because omnidiff mapped it to an old comment.
- **Insert -> None, 20 bytes, `matcher`.** The unchanged `--DOC_HIDE` of after lines 11 and 20 is
  painted Insert.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: `solve_leading_siblings` (`LeadingSibling`): do not treat a comment that
  starts on the end row of the previous sibling (a trailing comment) as leading the next statement
- Ground truth questionable: no
