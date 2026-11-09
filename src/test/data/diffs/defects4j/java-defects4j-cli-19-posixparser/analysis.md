# Accuracy analysis: java-defects4j-cli-19-posixparser

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 15 (9 visible) of 2054 nodes, 2064 graded, N:M floor 7 visible |
| Painting, Minimal | 54 bytes wrong of 20456: renderer 0, matcher 54 |
| Painting, Full | 82 bytes wrong of 20456: renderer 32, matcher 66 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-19 in `PosixParser.processOptionToken`: the two `tokens.add(token);` calls inside the
`if (options.hasOption(token))` and `else if (stopAtNonOption)` branches (before lines 232 and 237)
are replaced by a single unconditional `tokens.add(token);` after the `if` (after line 238), so an
unrecognised token is no longer dropped. Marko's `description.md`: "Requires N:M mapping."

## Mapping

- **The merged `tokens.add(token);` (7 visible, 3 invisible).** The ground truth maps both before
  copies (lines 232 and 237) to the one after copy as an all-to-all group (2 before <-> 1 after).
  omnidiff, being one-to-one, maps the line-237 copy and deletes the line-232 copy (reason
  `APTED("qualified_name")`). The N:M floor is exactly these 7 visible tokens. `nm-floor`.
- **The `else if` block's braces (2 visible, 3 invisible).** `{` (before line 235) and `}` (before
  line 238), plus the two `if_statement` nodes (lines 229 and 234) and the `else if` block. The
  ground truth keeps them; omnidiff leaves them unmatched (op Delete, `APTED("qualified_name")`)
  while the `if` keywords, conditions and remaining statements are matched. `matcher`.

## Painting

N:M-caused, Minimal 54 bytes (all of Minimal's `real`), Full 26 bytes:

- Delete -> Move: the line-232 copy, which omnidiff cannot also map (18 bytes Minimal, 26 Full
  with indentation).
- Minimal 36 bytes, None -> Move: the line-237 copy and the after line-238 copy. Both paintings
  paint the merge as `Move`. omnidiff's one-to-one pair 237 -> 238 keeps row order and is not
  painted `Move` under Minimal; with the human N:M group it is. `nm-floor`.

Matcher-caused, Full 36 bytes, Delete/Insert -> None: the `else if` block's `        {` and
`        }` (before lines 235, 238; after lines 234, 236), painted Delete and Insert because their
nodes are unmatched.

Renderer-caused (also with the human mapping), Full 20 bytes:

- None -> Delete, 12 bytes: the indentation of before line 237, which the Full painting deletes
  with the statement. `renderer`.
- None -> Move, 8 bytes: the indentation of after line 238, which the Full painting moves with
  the statement. `renderer`.

## Verdict

- Primary cause: `nm-floor`
- Other causes: `matcher`, `renderer`
- Where a fix would go: N:M support (`solve_multi_maps`) for two identical statements merged into
  one; `APTED("qualified_name")` for the unmatched `else if` braces
- Ground truth questionable: no
