# Accuracy analysis: tsx-mui-material-ui-add-name-to-pair

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 6 (4 visible) of 287 nodes, 287 graded, N:M floor 0 visible |
| Painting, Minimal | 74 bytes wrong of 909: renderer 19, matcher 59 |
| Painting, Full | 17 bytes wrong of 909: renderer 31, matcher 23 |
| Ground-truth invariant violations | 2 (rule [16], Minimal painting) |

## The change

Line 15 `inputProps={{ 'aria-label': 'controlled' }}` becomes `slotProps={{` / `input: {
'aria-label': 'controlled' },` / `}}` (after lines 15-17): the attribute is renamed and the old
object is nested under a new `input` key.

## Mapping

- Which object survives (4 visible, 2 invisible; `ambiguous`). The ground truth keeps the before
  object (`{ 'aria-label': 'controlled' }`, line 15 col 19) as the after OUTER object (the one
  holding `input:`), maps its `{`/`}` to the outer braces, moves the `'aria-label'` pair into a
  new inner object and inserts the inner object and its braces. omnidiff maps the before object
  to the after INNER object at line 16 col 16, which is byte-identical (`WrapGrowth`, braces by
  `APTED("qualified_name")`), and treats the outer object as new. Both readings are defensible:
  the ground truth follows the attribute value's `{{ }}`, omnidiff follows the identical text.

## Painting

Minimal (74 bytes):

- Matcher, 58 bytes (`ambiguous`): following its mapping, omnidiff leaves `'aria-label':
  'controlled'` unpainted on both sides (26 + 26) where the painting moves it, leaves the
  inner `{` / `},` on after line 16 unpainted where the painting inserts them (4), and leaves one
  outer `{` per side unpainted where the painting matches `={{` as Move (2).
- Ground truth, 10 bytes (`ground-truth`): the painting marks all of `inputProps` / `slotProps`
  as Update; invariant [16] flags `Props` (5 bytes per side) as wrong for a Minimal reading.
  omnidiff leaves `tProps` unpainted.
- Renderer, 6 bytes (`renderer`): the `t` before `Props` on each side (2) is left out because
  omnidiff narrows `inpu`/`slo` by characters, not words; the closing `}}` (2 per side) is moved
  to line 17 and the painting marks it Move, omnidiff leaves brackets unpainted.

Full (17 bytes):

- Renderer, 12 bytes (`renderer`, also with the human mapping): the 6-space indentation of line
  15 on both sides, None -> Move; the Full painting matches it to itself (same position, graded
  Move), omnidiff leaves it unpainted.
- Matcher, 5 bytes (`ambiguous`): the outer `{` on after line 15 and `}` on line 17 painted
  Insert (painting: Move), the inner `{` / `}` on line 16 left unpainted or Move (painting:
  Insert).

## Verdict

- Primary cause: `ambiguous`
- Other causes: `ground-truth`, `renderer`
- Where a fix would go: none for the object choice (`WrapGrowth` picks the identical inner
  object; the ground truth picks the outer one); word-level narrowing of identifier updates in
  src/diff/text.rs for the `t`
- Ground truth questionable: yes: the Minimal painting paints `Props` as part of the
  `inputProps`/`slotProps` update, which invariant [16] flags
