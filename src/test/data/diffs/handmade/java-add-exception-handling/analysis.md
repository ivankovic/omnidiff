# Accuracy analysis: java-add-exception-handling

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 6 (4 visible) of 331 nodes, 331 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 1094: renderer 0, matcher 4 |
| Painting, Full | 52 bytes wrong of 1094: renderer 44, matcher 44 |
| Ground-truth invariant violations | none |

## The change

Handmade fixture: the body of `readFile` is wrapped in `try { ... } catch (FileNotFoundException e)
{ ... }`. The six original statements move one level deeper, unchanged except for indentation.

## Mapping

- **Method body braces (4 visible).** Lines 6 and 14 of before: the method block's `{` and `}`.
  The ground truth keeps them on the method block's braces (after lines 6 and 19) and marks the
  `try` block's `{` (after line 7) and `}` (after line 15) as inserted. omnidiff maps the before
  method block to the after `try` block, op Identical, reason `IdenticalHashOfAncestor`: the
  before method block and the after `try` block contain the same five statements, so their
  hashes are equal and the hash pass pairs them, braces included. The outer method block on the
  after side is then left without a partner. The ground truth's reading (the method body stays,
  its statements go into a new `try`) is the natural one; the hash pass prefers content over the
  parent position. `matcher`.
- **`while` statement and its block (2 invisible).** Before line 9. Mapped to the right node, but
  the ground truth expects op MatchButNotIdentical (the text differs by indentation) and omnidiff
  says Identical (the hash ignores whitespace). No visible effect. `grading-artifact`.

## Painting

Matcher-caused (follow from the block mapping above):

- Minimal, 4 bytes, None -> Insert: after line 7 ` {` of `try {` and after line 15 `} ` of
  `} catch`. With the before method block mapped to the `try` block, those braces are matched, so
  omnidiff does not paint them inserted.
- Full, about 28 bytes: the same two braces, plus the method braces painted the wrong way round
  (before line 6 `{` and line 14 `}` painted Move, after line 6 `{` and line 19 `}` painted
  Insert, where the painting leaves them unpainted), and after line 15 `        } ` painted Move
  instead of Insert.

Renderer-caused (also present with the human mapping):

- Full, 24 bytes, Move -> Insert: the four added indentation spaces on after lines 9-14. The Full
  painting paints the indentation the `try` adds as Insert; omnidiff paints it as part of the
  moved statement. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`, `grading-artifact`
- Where a fix would go: the hash pass (`IdenticalHash` / its ancestor re-tag): a block whose
  identical twin is a new wrapper's block, while its own parent is matched, could keep its
  positional partner; `solve_wrap_growth` is the pass that handles wrappers.
- Ground truth questionable: no
