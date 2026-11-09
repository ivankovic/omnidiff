# Accuracy analysis: csharp-sonarr-sonarr-delete-func

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 232 nodes, 232 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 1175: renderer 2, matcher 0 |
| Painting, Full | 0 bytes wrong of 1175: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The commit removes the `using System.Runtime.Serialization;` import (before line 3) and the
serialization constructor `RootFolderNotFoundException(SerializationInfo, StreamingContext)` (before
lines 23-26).

## Painting

Minimal only, 2 bytes, identical with the human mapping, so renderer-owned:

- None -> Delete, before lines 25 and 26: the lone `{` and `}` of the deleted constructor's empty
  body. The Minimal painting includes them in the Delete; omnidiff's Minimal rendering drops a
  per-row range that is only structural punctuation (`is_structural_only`,
  src/diff/text/render_options.rs), and both halves of the pair were dropped. `renderer`.

Full agrees exactly.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter in
  src/diff/text/render_options.rs, for brace-only rows of a wholly deleted construct
- Ground truth questionable: no
