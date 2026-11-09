# Accuracy analysis: java-defects4j-cli-14-groupimpl

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 29 (19 visible) of 6390 nodes, 6390 graded, N:M floor 0 visible |
| Painting, Minimal | 119 bytes wrong of 32709: renderer 155, matcher 121 |
| Painting, Full | 243 bytes wrong of 32709: renderer 269, matcher 222 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-14 in `GroupImpl.validate`: the `if (validate) { option.validate(commandLine); }`
block (before lines 250-252) moves below the `if (commandLine.hasOption(option))` block (after
lines 261-263), and inside that block the trailing `option.validate(commandLine);` (before line
262) becomes `validate = true;` (after line 258). A present child is now validated once, after the
count check.

## Mapping

All through `APTED("qualified_name")`.

- **Which `option.validate(commandLine);` survives (17 visible, 8 invisible).** The text has two
  identical statements before and one after. The ground truth (and both paintings) moves the
  whole `if (validate)` block from lines 250-252 to 261-263 and replaces the inner call on
  before line 262 with `validate = true;`. omnidiff keeps the inner call on before line 262 in
  place as the body of the new `if (validate)` (after line 262), and deletes the old `if
  (validate) { ... }` (10 tokens) while inserting the new `if (validate) {` and `}`. The ground
  truth's account is one move and one replacement; omnidiff's is two deletions and two
  insertions around a kept line. `matcher`.
- **The `hasOption` block's braces (2 visible, 2 invisible).** Before line 255 `{` and line 263
  `}`, with the `if_statement` and `block` nodes. omnidiff leaves them unmatched although the
  `if` keyword, the condition and the block's other statements are matched. Having given
  before line 262 to the `if (validate)` body, the before block's `}` no longer lines up. The
  structure is broken. `matcher`.

## Painting

Matcher-caused, Minimal 119 bytes, Full 243 bytes (all of `real`):

- Delete/Insert -> Move: the `if (validate) { option.validate(commandLine); }` block on before
  lines 250-252 painted Delete and the new one on after lines 261-263 painted Insert where the
  paintings paint one `Move` (Minimal 90, Full about 170 bytes).
- None -> Delete: the inner `option.validate(commandLine)` (before line 262), unpainted because
  omnidiff keeps it (Minimal 28, Full 44).
- Full 28 bytes: the `hasOption` block's braces painted Delete/Insert (before lines 255, 263;
  after lines 252, 259); the painting leaves them unpainted. Plus 1 byte, the `;` of `validate =
  true;` painted Insert in both presets.

Seen only with the human mapping (not in `real`, but what a matcher fix would expose):

- Minimal 30 / Full 85 bytes, None -> Move: with the block correctly matched, omnidiff's renderer
  does not paint the moved `if (validate)` block `Move`. Instead it paints the comment `// if the
  child option is present then validate it` (line 254) `Move` (50 bytes, Move -> None): the
  walk-order attribution blames what the block jumped over (`move_attribution.md`). `renderer`.
- Minimal 15 / Full about 49 bytes: the human tree maps the receiver `option` (before line 262) to
  the assignment target `validate` (after line 258) as an Update, but both paintings delete
  `option.validate(commandLine)` and insert `validate`. The renderer follows the tree and paints
  Update. The tree and the paintings disagree; `invariants.txt` does not check Update against
  Delete/Insert. `ground-truth`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`, `ground-truth`
- Where a fix would go: `APTED("qualified_name")` twin choice between the two identical
  `option.validate(commandLine);` statements (prefer keeping the enclosing `if (validate)`
  matched); then `diff::text` move attribution, which would mis-paint the corrected mapping
- Ground truth questionable: yes: the tree pairs `option` (before line 262) with `validate`
  (after line 258) as an Update, while both paintings paint them Delete and Insert
