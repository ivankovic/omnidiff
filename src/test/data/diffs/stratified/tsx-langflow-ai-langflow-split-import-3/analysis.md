# Accuracy analysis: tsx-langflow-ai-langflow-split-import-3

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 10 (7 visible) of 189 nodes, 198 graded, N:M floor 6 visible |
| Painting, Minimal | 68 bytes wrong of 601: renderer 4, matcher 71 |
| Painting, Full | 48 bytes wrong of 601: renderer 10, matcher 45 |
| Ground-truth invariant violations | none |

## The change

`import React, { forwardRef } from "react";` is split into `import type React from "react";` and
`import { forwardRef } from "react";`. description.md: "Requires N:M mapping".

## Mapping

- N:M split (6 visible, 3 invisible; `nm-floor`). The ground truth has all-to-all groups: the one
  before `import_statement`, its `import`, `import_clause`, `"react"` string and `;` map to both
  after statements. omnidiff maps the before statement to after line 2 (`import { forwardRef }`,
  the one that shares more) and inserts after line 1's `import`, `from`, `"`, `react`, `"`, `;`
  plus the statement, clause and string nodes (reason `APTED("bottom_up_propagation")`). One
  one-to-one mapping cannot cover both targets.
- `React` (before line 1, col 8; 1 visible; `matcher`). The ground truth maps it identical to
  `React` in after line 1 (`import type React`). omnidiff deletes it
  (`APTED("import_path_similarity")`) and inserts the after `React`, because its parent
  statement went to after line 2. A cross-parent
  move of the identifier would satisfy this entry even under a one-to-one mapping.

## Painting

Both paintings mark the before `import` and `from "react"` as Match to both after lines (shown as
Move) and leave `React` unpainted (Minimal) or Match it (Full).

- Matcher, N:M (Minimal ~58 bytes, Full ~35; `nm-floor`): before `import` / `from "react"` and
  after line 2 `import` / `from "react"` None -> Move; after line 1 `import ` and `from "react";`
  Insert -> Move. omnidiff treats line 2 as the unchanged copy and line 1 as new.
- Matcher, `React` (about 10 bytes in each preset; `matcher`): before `React` Delete -> None and
  after `React` Insert -> None (Minimal) / Move (Full), following the mapping mismatch above.
- Renderer (Minimal 4, Full 10 with the human mapping; about 3 bytes overlap omnidiff's own
  runs; `renderer`): with the human mapping the renderer paints the `;` of each statement Move
  (the painting leaves it unpainted), leaves the deleted `,` unpainted in Minimal, and in Full
  leaves the single spaces between moved tokens unpainted where the Full painting has them Match.

## Verdict

- Primary cause: `nm-floor`
- Other causes: `matcher`, `renderer`
- Where a fix would go: none for the split itself (one-to-one limit); the `React` identifier
  could be recovered by a cross-parent move of an identical leaf after `import_path_similarity`
  pairs the statements
- Ground truth questionable: no
