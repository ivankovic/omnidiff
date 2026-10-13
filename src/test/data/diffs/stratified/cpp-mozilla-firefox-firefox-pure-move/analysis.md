# Accuracy analysis: cpp-mozilla-firefox-firefox-pure-move

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 49314 nodes, 49314 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 245702: renderer 0, matcher 0 |
| Painting, Full | 8 bytes wrong of 245702: renderer 8, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Two statements, `stream->AddIPDLReference();` and `handle->AddIPDLReference();`, each move up a few
lines in their function (before lines 2897 and 2923 to after lines 2890 and 2917). It is a pure
move.

## Painting

- **Indentation of the moved lines (Full, 8 bytes, `None -> Move`, `renderer`).** The Full painting
  paints the two leading spaces of each moved line Move, on both sides. Omnidiff's Full rendering
  paints the statement and leaves the indentation unpainted. The runs are the same with the human
  mapping.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full rendering in `src/diff/text.rs`. It would include the leading
  indentation in a whole-line Move.
- Ground truth questionable: no
