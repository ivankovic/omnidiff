# Accuracy analysis: java-defects4j-lang-19-numericentityunescaper

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 49 (34 visible) of 1124 nodes, 1169 graded, N:M floor 30 visible |
| Painting, Minimal | 110 bytes wrong of 6139: renderer 4, matcher 106 |
| Painting, Full | 180 bytes wrong of 6139: renderer 90, matcher 106 |
| Ground-truth invariant violations | none |

## The change

Defects4J Lang-19. `NumericEntityUnescaper.translate` tightens its bounds checks. The guard becomes
`index < seqEnd - 2`, and an `if(start == seqEnd) return 0;` is added after the hex check. The
`while(input.charAt(end) != ';')` loop becomes a bounded hex-digit scan with six `input.charAt(end)`
comparisons over three lines. The trailing `+ 1` of the return value becomes `(semiNext ? 1 : 0)`.
`description.md`: "Requires N:M mapping."

## Mapping

- **N:M floor (30 visible, 15 invisible), `nm-floor`.** The ground truth puts the single
  `input.charAt(end)` of the old loop condition (line 54) in an all-to-all group with all six copies
  in the new condition (lines 57-59). omnidiff maps it to the first copy and inserts the other five
  (`APTED("qualified_name")`). Each copy has 6 visible leaves (`input`, `.`, `charAt`, `(`, `end`,
  `)`), so 5 x 6 = 30, which is exactly the fixture's visible N:M floor. The 15 invisible ones are
  the five copies' `binary_expression`, `method_invocation` and `argument_list` nodes.
- **Operator and literal updated (4 visible), `ambiguous`.** In the old `input.charAt(end) != ';'`
  (line 54), omnidiff maps `!=` to `>=` and `';'` to `'0'` of the first new comparison
  `input.charAt(end) >= '0'` (line 57), op MatchButNotIdentical. The ground truth deletes `!= ';'`
  and inserts `>= '0'`. The comparison is matched in both mappings. Whether its operator and operand
  were edited or replaced is a judgement call: the test changed meaning, but the slot is the same.

## Painting

Matcher-owned bytes (official split: Minimal 106, Full 106):

- **N:M copies, `Insert -> Move/None`.** The five unmatched `input.charAt(end)` copies on lines
  57-59 are painted Insert. The painting has one Match entry that pairs the old span with all six
  new spans, so it shows them as carried over. This is about 93 bytes in Minimal and 85 in Full.
  `nm-floor`.
- **`!= ';'` / `>= '0'`, `Update -> Delete/Insert`.** 13 bytes on lines 54 and 57. `ambiguous`.

Renderer-owned bytes (they survive with the human mapping):

- Minimal, 4 bytes. On line 57 a space after `(` and on line 59 a space before the last `)` are
  painted Insert; the painting leaves interior spaces between inserted tokens unpainted. The `1` of
  the old trailing `+ 1` (line 80) and of `semiNext ? 1 : 0` (line 86) is painted Move; Minimal
  leaves it unpainted. `renderer`.
- Full, 90 bytes. The 36-column continuation indentation of lines 58 and 59 is painted Insert, where
  the Full painting leaves it unpainted (72 bytes). Several single spaces that the Full painting
  inserts between the new tokens are left unpainted, and on lines 80 and 86 the rendering paints `;`
  Move where the painting has ` 1` as a Match. `renderer`.

## Verdict

- Primary cause: `nm-floor`
- Other causes: `ambiguous`, `renderer`
- Where a fix would go: none for the N:M part (Marko's N:M ground truth is beyond a one-to-one
  mapping); `src/diff/text.rs` for Full continuation-indentation and inter-token spaces
- Ground truth questionable: no
