# Accuracy analysis: java-defects4j-math-103-normaldistributionimpl

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 6 (4 visible) of 1394 nodes, 1394 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 13343: renderer 0, matcher 4 |
| Painting, Full | 246 bytes wrong of 13343: renderer 0, matcher 246 |
| Ground-truth invariant violations | none |

## The change

Defects4J Math-103. `NormalDistributionImpl.cumulativeProbability` wraps its single `return`
statement in `try { ... } catch (MaxIterationsExceededException ex) { ... }`. The catch returns 0 or
1 for extreme `x`. The `return` line keeps its 12-space indentation, because the original file was
under-indented.

## Mapping

- **Method body matched to the try block (4 visible, 2 invisible), `matcher`.** The ground truth
  maps the old method body `block` (line 108 `{` to line 111 `}`) to the new method body
  (MatchButNotIdentical), and marks the try statement's `block` (line 109 `{`, line 112 `}`) as new.
  omnidiff maps the old body to the try's block: the two have the same text apart from whitespace,
  so they are Identical by hash. The braces follow (`IdenticalHashOfAncestor`). `solve_wrap_growth`
  then re-tags the block match as `WrapGrowth`. So the old method `{`/`}` map to the try block's
  `{`/`}`, and the new method's own braces count as inserted. The ground truth's reading is the
  structural one: the method body is still the method body, and the `return` gained a new parent.
  The hash match prefers the identical twin one level down.

## Painting

All runs are matcher-owned: rendering the human mapping gives 0 bytes in both presets.

- **Minimal, 4 bytes (`None -> Insert`).** The try block's ` {` (line 109) and `} ` (line 112) are
  left unpainted, because they hold the old method body's braces. The painting inserts them with
  `try` and `catch`.
- **Full, 246 bytes.** With the `WrapGrowth` tag, Full paints the whole unchanged `return` statement
  (lines 109-110 before, 110-111 after; 111 bytes per side) and the old method braces as Move. It
  also paints the new method's `{` (line 108) and closing `}` (line 121) as Insert. The Full
  painting leaves the return and the method braces unpainted, and paints only `try {` and the catch
  clause. Here the return was not even re-indented.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the identical-hash matching that pairs the old method body with the try
  block (re-tagged later by `solve_wrap_growth`); it should keep a method body on the method body
  when the identical twin is a new wrapper's block
- Ground truth questionable: no
