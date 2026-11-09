# Accuracy analysis: java-defects4j-cli-15-writeablecommandlineimpl

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 35 (25 visible) of 3767 nodes, 3767 graded, N:M floor 0 visible |
| Painting, Minimal | 369 bytes wrong of 17680: renderer 110, matcher 325 |
| Painting, Full | 632 bytes wrong of 17680: renderer 428, matcher 295 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-15 in `WriteableCommandLineImpl.getValues(Option, List)` (before lines 117-127,
after lines 117-135). The first `if` now fills `defaultValues` from `this.defaultValues` when it
is empty (its body `valueList = defaultValues;` becomes `defaultValues = (List)
this.defaultValues.get(option);`). The second `if ((valueList == null) || valueList.isEmpty())`
becomes the inner `if` of a new `if (defaultValues != null && !defaultValues.isEmpty())`, with a
new `else` that appends the extra defaults. The three stray comments (before lines 125-127) end up
inside that `else` (after lines 126-129).

## Mapping

- **The second `if` scattered (16 visible, 8 invisible), `APTED("qualified_name")`.** The ground
  truth maps the second `if` (before line 122) to the inner `if (valueList == null ||
  valueList.isEmpty())` (after line 123), with the same condition minus parentheses, and its
  body `valueList = (List) this.defaultValues.get(option);` to `valueList = defaultValues;`
  (after line 124). omnidiff spreads it over three different after constructs: the `if`
  keyword and statement go to the new outer `if` (after line 122), the block's `{` to the
  size-check `if`'s block (after line 128), the assignment's `valueList`, `=` and `;` to
  `valueList = new ArrayList(valueList);` (after line 130), with the cast type `List` paired
  with the variable `valueList` there, and the `}` to after line 134. The comment `// the list.`
  (before line 126) is deleted. `matcher`.
- **`(option)` moved alone (6 visible, 2 invisible), `MovedSubtree`.** The argument list `(option)`
  of `this.defaultValues.get(option)` on before line 123 is paired with the one on after line
  118, while the rest of both calls stays deleted and inserted, in omnidiff as in the ground
  truth. Moving a three-token fragment out of a deleted call is not a reading either way. (The
  whole `(List) this.defaultValues.get(option)` is byte-identical on both lines and could be
  read as moved; the ground truth reads it as deleted and inserted.) `matcher`.
- **The first `if`'s assignment (3 visible), `APTED("qualified_name")`.** `valueList =
  defaultValues;` (before line 118) -> `defaultValues = (List) this.defaultValues.get(option);`.
  The ground truth pairs the right-hand `defaultValues` with the new left-hand `defaultValues`
  and deletes `valueList`. omnidiff renames `valueList` to `defaultValues` (left to left) and
  pairs the right-hand `defaultValues` with the field name in `this.defaultValues`. Both pair a
  name with a same-named token. `ambiguous`.

## Painting

The runs overlap too much to separate byte for byte; the table's renderer/matcher split is the
reference.

Matcher-caused (Minimal about 325 bytes, Full about 295):

- Minimal 190 bytes, Move/Delete/Insert -> None: the three comments (before 125-127, after
  126-129). The Minimal painting leaves them unpainted, and so does omnidiff's renderer with the
  human mapping. With omnidiff's scattered mapping they are crossed by matched pairs and painted
  `Move`, and `// the list.` is deleted and inserted.
- The second `if`'s pieces: after `if `, ` {`, `valueList = `, `valueList`, `;`, `}` (lines
  122-134) unpainted or `Update`/`Move` where the painting inserts them, and before `(valueList
  == null)`, `List`, `(option)` where the painting deletes them.

Renderer-caused (also with the human mapping, Minimal 110 bytes, Full 428):

- Minimal 89 bytes, Move -> None: the re-nested `if (valueList == null` and `valueList =
  defaultValues` (before 122-123, after 123-124). They changed parent because a new `if` wraps
  them, which `MINIMAL` should not paint, but the `WrapGrowth` re-tag that would suppress it
  does not apply because the wrapped `if` itself changed. `renderer`.
- Minimal 14 bytes, None -> Move/Insert: the `defaultValues` that went from right-hand to
  left-hand side on line 118, which the painting paints `Move`, and the new `;`. `renderer`.
- Full 294 bytes, None -> Move: the three comments, which the Full painting pairs as `Move` (they
  shifted rows and changed blocks) and omnidiff, with either mapping, does not. `renderer`.
- Full about 100 bytes, None -> Move: `        if (`, ` `, `.isEmpty()) {`, the indentation of
  line 118 and `        }` of the first `if` (lines 117-119). The Full painting matches these
  spans to themselves at the same row and column (e.g. `117:0-117:12 -> 117:0-117:12`, `119:0-9 ->
  119:0-9`), and a byte-identical match is scored as `Move`. Nothing there moved. `ground-truth`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`, `ambiguous`, `ground-truth`
- Where a fix would go: `APTED("qualified_name")`, which scatters one `if` over three after
  constructs when a new wrapper `if` appears; `solve_moved_subtrees` (`MovedSubtree`) should not
  move a fragment out of a deleted parent; `diff::text` moves for re-nested and re-parented
  comments
- Ground truth questionable: yes: the Full painting matches unchanged text on lines 117-119 to
  itself at the same position, which scores as about 100 bytes of `Move` where nothing moved
