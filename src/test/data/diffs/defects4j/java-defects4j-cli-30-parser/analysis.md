# Accuracy analysis: java-defects4j-cli-30-parser

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 31 (21 visible) of 3502 nodes, 3502 graded, N:M floor 0 visible |
| Painting, Minimal | 274 bytes wrong of 27409: renderer 222, matcher 92 |
| Painting, Full | 400 bytes wrong of 27409: renderer 210, matcher 194 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-30, the same fix as `java-defects4j-cli-30-defaultparser` applied to
`Parser.processProperties`: `Option opt = getOptions().getOption(option);` (before line 265) is
hoisted above the `if` as `Option opt = options.getOption(option);` (after line 263) with a new
`UnrecognizedOptionException` guard, `OptionGroup group` / `boolean selected` lines are added, and
`if (!cmd.hasOption(option))` (before line 263) becomes `if (!cmd.hasOption(option) && !selected)`
(after line 273). The comment `// if the option is part of a group ...` ends up outside the `if`
(before line 267 -> after line 269).

## Mapping

All through `APTED("qualified_name")`.

- **The two calls swapped (21 visible, 9 invisible).** The ground truth maps the old `if` (before
  lines 263-264, 297) to the grown `if` (after lines 273-274, 303) and the old declaration
  (before line 265) to the hoisted one (after line 263). omnidiff crosses the two calls: the
  condition's `hasOption(option)` (before 263) is paired with the hoisted `getOption(option)`
  (after 263) and the declaration's `getOption(option)` (before 265) with the new condition's
  `hasOption(option)` (after 273), each as an Update of the method name. Around them, the old
  `if` header and braces and the declaration's `Option opt =` and `;` are deleted, and their
  counterparts inserted. The pairing follows row position (each call is paired with whatever
  call now sits on its row), not structure. `matcher`.
- **`getOptions()` -> `options` (1 invisible).** A 2:1 group the ground truth expects one pair
  from; omnidiff pairs none. No visible effect. `grading-artifact`.

## Painting

Matcher-caused (Minimal 90 / Full 194 bytes): the old `if (!cmd.has` ... `)` and braces painted
Delete and the new ones Insert; `Option opt = `, `.getOption` and `;` painted Delete/Insert on both
declarations where the paintings paint them `Move`.

Renderer-caused (also with the human mapping):

- Minimal 182 / Full 206 bytes, None -> Move: the comment `// if the option is part of a group,
  check if another option of the group has been selected` (before line 267, after line 269),
  which left the `if` block. Not painted `Move` with either mapping, as in the `DefaultParser`
  fixture. `renderer`.
- Minimal 2 bytes, None -> Insert: `{` and `}` of the new `throw` block (after lines 265, 267).
  `renderer`.

Seen only with the human mapping: `.getOption(option);` of the hoisted declaration (Minimal 38
bytes), which the paintings paint `Move`, is not painted `Move`; with the human mapping Full is off
only by the comment and three single bytes. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`, `grading-artifact`
- Where a fix would go: `APTED("qualified_name")`: keep the `if` whose condition only grew and
  the declaration that was only hoisted, instead of pairing calls by row; `diff::text` move
  painting for comments that leave a block
- Ground truth questionable: no
