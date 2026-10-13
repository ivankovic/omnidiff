# Accuracy analysis: tsx-microsoft-typescript-libpath-to-lib

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 214 nodes, 214 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 625: renderer 2, matcher 0 |
| Painting, Full | 4 bytes wrong of 625: renderer 4, matcher 0 |
| Ground-truth invariant violations | none |

## The change

One test-directive comment on line 4 changes from `// @libFiles: react.d.ts` to
`/// <reference path="/.lib/react.d.ts" />`. The comment is one node, mapped as an update.

## Painting

All runs are the same with the human mapping, so they are the renderer's.

- Minimal, 2 bytes, after line 4 cols 2-3 (`/ `), Update -> Insert/None (`renderer`). The
  painting inserts the third `/` and leaves the following space unpainted; omnidiff's narrowed
  update of the comment starts at col 2 and paints `/ ` as Update.
- Full, 4 bytes, `//` at line 4 cols 0-1 on both sides, None -> Move (`renderer`). The Full
  painting matches `//` to `//` as its own entry (identical text, so it is graded as Move, though
  it is at the same row and column); omnidiff's narrowed update leaves the unchanged prefix
  unpainted.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: update narrowing of comment text in src/diff/text.rs (where the
  inserted `/` and the space fall); the Full run is a convention question about painting an
  unchanged same-position prefix
- Ground truth questionable: no
