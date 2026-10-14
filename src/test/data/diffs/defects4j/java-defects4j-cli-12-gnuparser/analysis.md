# Accuracy analysis: java-defects4j-cli-12-gnuparser

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 4 (2 visible) of 991 nodes, 991 graded, N:M floor 0 visible |
| Painting, Minimal | 8 bytes wrong of 7793: renderer 86, matcher 90 |
| Painting, Full | 222 bytes wrong of 7793: renderer 194, matcher 30 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-12 in `GnuParser.flatten`: a new first branch `if (opt.indexOf('=') != -1 &&
options.hasOption(...)) { ... }` (after lines 81-86) handles `--foo=value`, and the old `if
(options.hasOption(arg.substring(0, 2)))` (before line 81) becomes its `else if` (after line 87).
The comment `// the format is --foo=value or -foo=value` (line 83) now sits in the new branch.

## Mapping

- **The old `if` statement and its keyword (2 visible, 2 invisible).** Before line 81. The ground
  truth maps the old `if_statement` to the `else if` at after line 87 and inserts the new outer
  `if`. omnidiff, reason `APTED("qualified_name")`, maps the old `if_statement` node and its `if`
  keyword to the new outer `if` at the same position (line 81), while its condition and block go
  to the `else if`, as in the ground truth. One statement is split across two. `matcher`.

## Painting

Matcher-caused (absent with the human mapping), Minimal 6 bytes, Full about 29: the `if` keyword
swap. After line 81 `if ` is not painted Insert and after line 87 ` if` is painted Insert; under
Full also the indentation of after line 81, and the old `if` of before line 81 not painted
`Move`.

Renderer-caused (also with the human mapping), Minimal 2 bytes, Full about 193:

- Full 132 bytes, None -> Move: the comment `// the format is --foo=value or -foo=value` (line 83,
  both sides). The painting paints it `Move` (it changed blocks). omnidiff does not, because its
  row order is unchanged. Under Minimal, omnidiff's own mapping happens to paint it `Move`, so
  this run appears only with the human mapping there (84 bytes). `renderer`.
- Full 40 bytes, None/Insert -> Move: the indentation of the old `if` line (before line 81) and
  of the `else if` line (after line 87), which the Full painting paints as moving with the `if`.
  `renderer`.
- Full 21 bytes, Move -> None: `{` of the `else if` block (after line 88), painted `Move` by
  omnidiff; the Full painting leaves it unpainted. `renderer`.
- Minimal 2 bytes, None -> Insert: the `{` and `}` of the new branch (after lines 82 and 86),
  which the Minimal painting paints Insert and `MINIMAL` leaves unpainted as structural
  punctuation. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`
- Where a fix would go: `diff::text` move painting for a statement or comment that changed parent
  without changing row order; `APTED("qualified_name")` for the `if` / `else if` split
- Ground truth questionable: no
