# Accuracy analysis: c-sqlite-sqlite-small-but-complex-case

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 115042 nodes, 46 graded, N:M floor 0 visible |
| Painting, Minimal | 434 bytes wrong of 593882: renderer 601, matcher 435 |
| Painting, Full | 562 bytes wrong of 593882: renderer 952, matcher 733 |
| Ground-truth invariant violations | none |

The tree ground truth has 29 entries and grades 46 of 115042 nodes, so the near-perfect mapping
score says little. The renderer and matcher shares are also not usable here: rendering the human
mapping is *worse* than omnidiff's own rendering (601 > 434, 952 > 562). For example, the new
`WHERETRACE` block at lines 5733-5743 is painted Insert, and the human-mapping rendering leaves it
unpainted because the sparse tree file does not list those nodes. The groups below therefore
classify omnidiff's own 434/562 bytes from the source text. They do not use the subtraction.

## The change

`src/where.c`, the star-schema heuristic. The commit rewrites the header comment as constraints
(aa)-(dd). It lowers the thresholds (`nLoop>=5` -> `>=4`, `iFromIdx+4` -> `+3`, `nDep<=3` ->
`<=2`) and puts two one-line `if` bodies into braces with new comments. It adds a `WHERETRACE`
block that prints the fact table and its dimensions (after lines 5727-5743). It also shortens the
"Increase SCAN cost" trace message. That last edit moves its `pFactTab...`, `iFromIdx`,
`pWLoop->iTab` and `pDim...` arguments into the new trace block's `printf` calls.

## Mapping

- Before line 5679 `/* Impossible to reach nDep>=4 */` (trailing `break;`). The ground truth maps
  it as an update to after line 5689 `/* ^-- Impossible to reach nDep>=2 - Constraint (cc) */`,
  which now sits inside the new braces. omnidiff deletes the old comment and inserts the new one,
  through `APTED("qualified_name")`. The two comments share "Impossible to reach nDep>=", so the
  update is reasonable. `matcher` (1 visible).

## Painting

Matcher-caused (omnidiff's mapping):

- The comment above. Delete/Insert where both paintings have Update: before line 5679 and after
  line 5689, 76 bytes in each preset.
- `iFromIdx` (before line 5742 -> after line 5732). The painting moves it into the new
  "Fact table" `printf`. omnidiff deletes the old one and inserts the new one (Delete/Insert ->
  Move). This is 17 bytes in Minimal and 41 in Full, including indentation. The tree ground truth
  does not grade this node.

N:M (`nm-floor`):

- Both paintings match `pDim->zAlias ? pDim->zAlias: pDim->pSTab->zName,` on before line 5740 to
  two after copies, line 5738 (new trace block) and line 5767, and paint all three Move. omnidiff
  keeps the before copy at line 5767 (unpainted) and paints line 5738 Insert. A one-to-one mapping
  cannot express the copy. This is 144 bytes in Minimal and 186 in Full with indentation.

Renderer-caused:

- Minimal: `pFactTab->zAlias ? ... zName,` (before line 5741 -> after line 5731) and
  `pWLoop->iTab` (before line 5740 -> after line 5738) are left unpainted on both sides where the
  painting has Move (146 bytes). The before side is not painted Delete, so these nodes are mapped
  across into the new `printf` calls. In Full, omnidiff paints `pFactTab` Move. The Minimal-only
  relaxation in `identical_or_move` (`shifted_by_an_edit_beside_it` with
  `node_untouched_on_its_row`, where the row index may differ under Minimal) appears to treat these
  unchanged-looking rows as displaced, not moved. They actually moved about ten lines up into a
  different call.
- The old trace string `"Increase SCAN cost of dimension %s(%d) of fact %s(%d) to %d\n"` (before
  line 5739). The painting Deletes `dimension` and `(%d) of fact %s(%d)` and keeps ` %s` and
  ` to %d` (Move in Full). omnidiff paints one Update span on the before side and `%s` Update on
  after line 5766. This is 33 bytes in Minimal and 54 in Full.
- Minimal: the comment-delimiter bytes `/* ` and ` */` of the updated comment are left unpainted
  where the painting includes them (12 bytes). The new braces `{`/`}` on after lines 5688, 5690,
  5711 and 5713 are dropped by the punctuation filter, while the painting marks them Insert (4
  bytes).
- Both presets: the header-comment update stops one byte short. The common suffix of
  `...only be joined against fact tables.` and `...are joined against fact tables.` starts inside
  the words `be`/`are` at `e`, so `e` is left out on before line 5596 and after line 5605 (2 bytes).
- Full only: the Full painting paints the kept parts of the two restructured `if` lines Move. These
  are `if( iFromIdx+4 > nLoop ) break;` -> `if( iFromIdx+3 > nLoop ){ break; ... }` (before line
  5679, after lines 5688-5689) and `if( nDep<=3 ) continue;` -> `if( nDep<=2 ){ continue; ... }`
  (before line 5703, after lines 5711-5712), including the reindented `break;`/`continue;` and
  their indentation. omnidiff leaves them unpainted (about 135 bytes). The Full `mxRun` argument
  is painted Move where the painting has None (10 bytes). Indentation and `,` around the moved
  arguments differ (55 bytes), and there are 3 bytes of whitespace on after lines 5658 and 5687.

## Verdict

- Primary cause: `renderer`
- Other causes: `nm-floor`, `matcher`, `grading-artifact`
- Where a fix would go: `src/diff/text.rs` `identical_or_move`. Under Minimal it should not count
  a node that moved into a different parent many rows away as displaced. Also the narrow-update
  rendering of rewritten string literals (Delete-within-string versus one Update span) and the
  in-word common-suffix narrowing of comment updates. On the matcher side, `APTED("qualified_name")`
  for the rewritten trailing comment and the unpaired `iFromIdx` argument.
- Ground truth questionable: no. The tree file is very sparse (46 graded nodes), which makes the
  mapping score and the renderer/matcher attribution unreliable here. Nothing in it is
  inconsistent, and there are no invariant violations.
