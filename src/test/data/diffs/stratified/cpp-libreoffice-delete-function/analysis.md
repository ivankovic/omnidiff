# Accuracy analysis: cpp-libreoffice-delete-function

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1007 nodes, 1007 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 7138: renderer 2, matcher 0 |
| Painting, Full | 0 bytes wrong of 7138: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The commit deletes the method `KFSalInstance::hasNativeFileSelection()` (before lines 63-70). It is
a pure deletion.

## Painting

- **Braces of the deleted function (Minimal, 2 bytes, `None -> Delete`, `renderer`).** The `{`
  (before line 64) and `}` (before line 69) stand on their own lines. Omnidiff's Minimal rendering
  drops them as structural punctuation. The Minimal painting paints them Delete with the rest of the
  function: in a wholly deleted construct the brackets share the construct's fate. The runs are the
  same with the human mapping.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter in `src/diff/text.rs`. It would
  keep the brackets of a construct that is deleted or inserted as a whole.
- Ground truth questionable: no
