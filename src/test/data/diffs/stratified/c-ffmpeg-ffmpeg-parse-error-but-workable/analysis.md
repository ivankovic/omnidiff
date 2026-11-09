# Accuracy analysis: c-ffmpeg-ffmpeg-parse-error-but-workable

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 34 (23 visible) of 62432 nodes, 62432 graded, N:M floor 0 visible |
| Painting, Minimal | 121 bytes wrong of 225108: renderer 24, matcher 145 |
| Painting, Full | 125 bytes wrong of 225108: renderer 24, matcher 129 |
| Ground-truth invariant violations | none |

## The change

FFmpeg `libavcodec/mjpegdec.c`: in one function, `if (s->restart_interval == 0)
s->restart_interval = INT_MAX;` (before lines 1146-1147) is removed, and the loop
`for (i = 0; i < 6; i++) vpred[i] = 1 << (s->bits-1);` moves from inside an
`if ((mb_y * s->width) % s->restart_interval == 0) { ... }` 25 lines further down (before lines
1172-1175, removed) to where the old `if` was (after lines 1146-1147). `description.md`: "Failed
to parse"; none of the node paths in the mismatches contain an `ERROR` node, so the parse errors
are not involved here.

## Mapping

- **The moved `for` loop, 18 visible + 8 invisible, and the removed assignment, 5 visible + 3
  invisible.** Ground truth moves the loop (before 1173-1174 -> after 1146-1147) and deletes
  `s->restart_interval = INT_MAX;`. omnidiff deletes the loop at its old place and maps the old
  `if` body `s->restart_interval = INT_MAX;` onto `vpred[i] = 1 << (s->bits-1);` in place
  (`s`, `->`, `=`, `;` Identical, `restart_interval` -> `bits` Update), all
  `APTED("qualified_name")`. The function-level APTED pairs the statement at the same position
  before any move detection sees the loop: the loop's new copy is reached as part of an in-place
  edit, so it is never a free, byte-identical insert for `MovedSubtree` to pair. `matcher`.

## Painting

- **Matcher, 121 bytes Minimal / ~109 Full:** the loop painted Delete at the old place and
  Insert / Update at the new place where the painting has Move both ways (`Delete->Move`,
  `Insert->Move`, `Update->Move`, `None->Move`), and `s->restart_interval = ` painted as part of
  an in-place edit where the painting deletes it. `matcher`.
- **Renderer, 24 bytes per preset (also with the human mapping):** indentation. Minimal paints
  the indentation inside the moved two-line range Move (16 + 8 bytes), the painting does not;
  Full leaves the old indentation of the moved lines unpainted where the Full painting deletes it
  (the loop went from 12/16 to 4/8 spaces). `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: move detection before the `qualified_name` APTED call site
  (`solve_moved_subtrees` for a statement whose identical copy exists elsewhere in the same
  function); indentation of moved multi-line ranges in `diff::text`
- Ground truth questionable: no
