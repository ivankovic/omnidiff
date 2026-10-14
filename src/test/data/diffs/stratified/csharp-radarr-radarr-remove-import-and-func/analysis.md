# Accuracy analysis: csharp-radarr-radarr-remove-import-and-func

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 291 nodes, 291 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 1128: renderer 2, matcher 0 |
| Painting, Full | 0 bytes wrong of 1128: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The commit removes the `using FluentValidation.Validators;` import (before line 2) and the extension
method `NotListenAllIp4Address<T>` (before lines 14-17).

## Painting

Minimal only, 2 bytes, identical with the human mapping, so renderer-owned:

- None -> Delete, before lines 15 and 17: the lone `{` and `}` of the deleted method's body. The
  Minimal painting includes them in the Delete; omnidiff's Minimal rendering drops a per-row range
  that is only structural punctuation (`is_structural_only`, src/diff/text/render_options.rs), and
  `restore_paired_brackets` does not restore a pair whose halves were both dropped. `renderer`.

Full agrees exactly.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter in
  src/diff/text/render_options.rs, for brace-only rows of a wholly deleted construct
- Ground truth questionable: no
