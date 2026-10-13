# Accuracy analysis: java-defects4j-cli-30-defaultparser

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 46 (32 visible) of 6178 nodes, 6178 graded, N:M floor 0 visible |
| Painting, Minimal | 341 bytes wrong of 40822: renderer 262, matcher 145 |
| Painting, Full | 473 bytes wrong of 40822: renderer 281, matcher 248 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-30 in `DefaultParser.handleProperties`: `Option opt = options.getOption(option);`
(before line 152) is hoisted above the `if` (after line 150) with a new
`UnrecognizedOptionException` guard; new `OptionGroup group = options.getOptionGroup(opt);` and
`boolean selected = ...` lines follow; and `if (!cmd.hasOption(option))` (before line 150) becomes
`if (!cmd.hasOption(option) && !selected)` (after line 160). The comment `// if the option is part
of a group ...` ends up outside the `if` (before line 154 -> after line 156).

## Mapping

All through `APTED("large_flat_subtree")`, the after-side lines included.

- **The hoisted declaration paired with the new `OptionGroup` line (20 visible, 8 invisible).** The
  ground truth maps `Option opt = options.getOption(option);` (before line 152) to the identical
  hoisted line (after line 150). omnidiff maps it to `OptionGroup group =
  options.getOptionGroup(opt);` (after line 157) with four Updates (`Option` -> `OptionGroup`,
  `opt` -> `group`, `getOption` -> `getOptionGroup`, `option` -> `opt`), and inserts the hoisted
  line. An identical line was available; the edit-distance fallback picked the near one by
  position. `matcher`.
- **The `if` (12 visible, 6 invisible).** The ground truth maps `if (!cmd.hasOption(option))` with
  its block braces (before lines 150, 151, 176) to the new `if (!cmd.hasOption(option) &&
  !selected)` (after lines 160-182). omnidiff deletes `if`, `(`, `!`, `cmd`, `.`, the outer `)`,
  `{` and `}`, and pairs only `hasOption` (as an Update) and `(option)` elsewhere. `matcher`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Painting

Matcher-caused (Minimal 157 / Full 267 bytes): the old `if` header and braces painted Delete and
the new ones Insert; the hoisted declaration painted Insert (after line 150) where the painting
moves it, and the old one painted as an Update into the `OptionGroup` line; the `OptionGroup`
line (after line 157) not painted Insert.

Renderer-caused (also with the human mapping):

- Minimal 182 / Full 206 bytes, None -> Move: the comment `// if the option is part of a group,
  check if another option of the group has been selected` (before line 154, after line 156). It
  left the `if` block for the loop body. omnidiff does not paint it `Move` with either mapping;
  its row order relative to its neighbours is unchanged. `renderer`.
- Minimal 2 bytes, None -> Insert: `{` and `}` of the new `throw` block (after lines 152, 154).
  `renderer`.

Seen only with the human mapping (not in `real`, but what a matcher fix would expose): the
hoisted declaration (Minimal 78 bytes) and, under Full, the `!cmd.hasOption(option)` condition
(47 bytes) that the paintings paint `Move` are not painted `Move` by omnidiff's renderer.
`renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: `APTED("large_flat_subtree")` (`solve_large_flat_subtrees`): prefer the
  identical hoisted declaration over a four-Update neighbour, and keep the `if` whose condition
  only grew; `diff::text` move painting for statements and comments that change parent block
- Ground truth questionable: no
