# Accuracy analysis: tsx-mui-material-ui-add-to-empty-block

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 23 (14 visible) of 62 nodes, 62 graded, N:M floor 0 visible |
| Painting, Minimal | 21 bytes wrong of 104: renderer 0, matcher 21 |
| Painting, Full | 29 bytes wrong of 104: renderer 6, matcher 28 |
| Ground-truth invariant violations | none |

## The change

`describe('<CssBaseline />', () => {});` gets a body: the empty block now holds
`it('To do', () => {});` (after line 2), and `});` moves to after line 3.

## Mapping

- Wrong twin for the empty arrow function (14 visible, 9 invisible; `matcher`). The ground truth
  keeps the before `() => {}` (line 1 col 29) as the `describe` callback, whose block grew, and
  inserts the whole `it(...)` statement. The new inner callback on after line 2 col 15 is also
  `() => {}`, byte-identical to the before one, so `IdenticalHash` maps the before arrow function
  to it and `IdenticalHashOfAncestor` carries its `(`, `)`, `=>`, `{`, `}` along (7 + 7 visible
  leaves on both sides). `APTED("fast_fallback")` then pairs the before `)` and `;` (line 1 cols
  37-38) with those of the inner `it(...)` statement, and the before `expression_statement`,
  `call_expression` and `arguments` are left `UnresolvedNode` (deleted, invisible). The identical
  hash match wins over the ancestor-consistent pairing even though the after twin sits inside an
  inserted statement whose own parent is the before callback's counterpart.

## Painting

- Matcher (Minimal 21 bytes, Full about 25; `matcher`): after line 1 `() => {` and line 3 `});`
  painted Insert (the painting leaves them unpainted, or moves `}` in Full), while most of the new
  `it('To do', () => {});` on line 2 (` () => {});`) is unpainted where the painting inserts it.
  In Full the before `() => ` and `);` are also painted Move.
- Renderer (Full, about 4 bytes; `renderer`): with the human mapping the renderer paints the
  `);` that moved from line 1 to line 3 Move on both sides; the Full painting moves only `}` and
  matches `{` to itself (graded Move) where omnidiff leaves it unpainted.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: `IdenticalHash` twin selection: an identical subtree whose after twin
  lies inside an inserted sibling of the before node's own grown parent should not beat the
  ancestor-consistent match (cf. the empty-block growth case)
- Ground truth questionable: no
