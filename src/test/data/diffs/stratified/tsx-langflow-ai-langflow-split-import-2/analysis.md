# Accuracy analysis: tsx-langflow-ai-langflow-split-import-2

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 189 nodes, 148 graded, N:M floor 0 visible |
| Painting, Minimal | 68 bytes wrong of 471: renderer 59, matcher 37 |
| Painting, Full | 48 bytes wrong of 471: renderer 105, matcher 93 |
| Ground-truth invariant violations | none |

## The change

`import React, { forwardRef } from "react";` is split into a type-only import and a value import,
`import type React from "react";` and `import { forwardRef } from "react";` (before line 1 / after
lines 1-2), in langflow's `Exa` icon (the same commit and edit as
`tsx-langflow-ai-langflow-split-import`). Marko's description.md: "Requies N:M mapping". The
paintings match `import`, `from "react"` (and in Full spaces) of the one before statement to both
after statements.

## Mapping

No mapping mismatches, but the result is vacuous for the edit: `human_mapping.json` has entries only
for the root, the unchanged `import SvgExa …` and the `export` statement. The changed import (both
after statements, 41 nodes) is ungraded, which is the gap between `graded` (148) and `nodes` (189).
For the same reason, rendering the human mapping paints nothing on lines 1-2, so the census's
renderer/matcher split (renderer bytes above the real bytes in Full) does not mean much here; the
groups below are read from omnidiff's own runs and the paintings.

omnidiff pairs the before import with the after `import { forwardRef } from "react";` (line 2),
inserts `import type React from "react";` (line 1) whole, and deletes the before `React`.

## Painting

Minimal, 68 bytes:

- N:M painting, 58 bytes (Move -> None and Insert -> Move): the painting matches `import` and
  `from "react"` 1:2 and labels all three copies Move. One-to-one, omnidiff can pair them with one
  after statement only: the line-2 copies and the before copies render unchanged (None, 36 bytes),
  and the line-1 copies are part of the inserted statement (Insert, 22 bytes). `nm-floor`.
- `React`, 10 bytes (Delete / Insert -> None): the Minimal painting leaves `React` unpainted on
  both sides (it only slid from column 7 to column 12), and the Full painting matches it. omnidiff
  deletes it from the before statement and re-inserts it with line 1, although it is identical and
  could be mapped across statements as a move. The node is ungraded in the tree ground truth, so
  only the painting sees this. `matcher`.

Full, 48 bytes:

- N:M painting, 35 bytes: `import ` on the before line and after line 2 (None vs Move, 14), and
  `import ` plus ` from "react";` on after line 1 (Insert vs Move, 21). `nm-floor`.
- `React`, 10 bytes (None/Insert vs Move): as above. `matcher`.
- 1 byte, None -> Delete: the space after the deleted `,` on the before line, which the Full
  painting deletes and omnidiff does not paint. `renderer`.
- 2 bytes, Move -> None: the `;` of the before line and of after line 2. omnidiff's Full rendering
  paints it Move (it changed column and row); this Full painting has no entry for `;`. The twin
  `tsx-langflow-ai-langflow-split-import`'s Full painting matches the same `;` 1:2 (so it is Move
  there and agrees with omnidiff). The two paintings of an identical edit differ here.
  `ground-truth`.

## Verdict

- Primary cause: `nm-floor`
- Other causes: `matcher`, `grading-artifact`, `renderer`, `ground-truth`
- Where a fix would go: none for the N:M part (one-to-one cannot express it); carrying the identical
  `React` into the new statement as a move would remove the matcher bytes. The tree ground truth
  does not cover the changed import at all, which is Marko's call
- Ground truth questionable: yes, mildly: the tree ground truth does not grade the changed import at
  all (41 of 189 nodes), so the 0 mismatches say nothing about the edit; and this Full painting
  leaves `;` unpainted where the twin `tsx-langflow-ai-langflow-split-import` matches it 1:2, for
  the identical edit
