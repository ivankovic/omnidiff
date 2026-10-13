# Accuracy analysis: json-louislam-uptime-kuma-big-add-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 86 (81 visible) of 326173 nodes, 326173 graded, N:M floor 0 visible |
| Painting, Minimal | 13 bytes wrong of 1714710: renderer 1, matcher 12 |
| Painting, Full | 859 bytes wrong of 1714710: renderer 653, matcher 206 |
| Ground-truth invariant violations | none |

## The change

`package-lock.json` (20k lines) gains three dev dependencies, pure additions: three lines in
`packages[""].devDependencies` (`eslint-config-prettier`, `prettier`, `stylelint-config-prettier`)
and three `node_modules/...` entries of 16-17 lines each. The single painting (`Minimal`) is
scored under both presets.

## Mapping

All 86 mismatches are before-side nodes deleted by `APTED("large_flat_subtree")` (the APTED run
over the largest flat container of the whole-file value, in `solve_large_flat_subtrees`), in the
two containers that received the new keys:

- the `""` package pair (before line 7): its key string `""` and quotes, `:`, the object's `{`,
  the commas after `name`, `version`, `license` (lines 8-10) and the closing `}` (line 169);
- the `devDependencies` pair (before line 95): `:`, `{`, and the comma after every entry on lines
  94-165 (73 commas in total, with lines 8-10 and 94).

The ground truth keeps all of them (Identical punctuation, MatchButNotIdentical pairs and
objects). omnidiff matches the unchanged entries inside both containers (no mismatch is reported
on them), so it keeps the children while deleting the containers and every separator between
them, and their after-side counterparts appear inserted. 81 visible (73 commas, two each of `"`,
`:`, `{`, `}`), 5 invisible (two pairs, two objects, one string). The pass that should have kept
the separators was not determined from the evidence; the same shape (containers and punctuation
deleted around matched children) appears in other JSON fixtures in the same pass family.
`matcher`.

## Painting

### Minimal, 13 bytes

- Matcher, 12 bytes: `"": {` (line 7) and its `}` (before line 169, after line 172) painted
  Delete on the before side and Insert on the after side; the painting leaves them unchanged.
  The 73 deleted commas do not show, because Minimal does not paint lone punctuation.
- Painting, 1 byte: the opening `"` of `"node_modules/eslint-config-prettier"` (after line 9950,
  column 8). omnidiff paints it Insert with the rest of the new entry; the painting's Insert
  starts at column 9, one character late, while its other two `node_modules` inserts start on the
  quote (column 8). `ground-truth`.

### Full, 859 bytes

- Matcher, 206 bytes: the deleted/inserted commas (144 runs, one byte each), `"": {` and
  `: {` with their indentation, and the closing braces, all painted Delete/Insert.
- 653 bytes, the same with the human mapping: the indentation of the 52 inserted lines (and the
  quote above) painted Insert by the Full rendering, against a Minimal painting that never paints
  indentation. `grading-artifact`.

## Verdict

- Primary cause: `matcher`
- Other causes: `grading-artifact`, `ground-truth`
- Where a fix would go: `solve_large_flat_subtrees` (`large_flat_subtree`) or a later
  parent-propagation pass: a JSON object/pair whose entries are matched, and the commas between
  them, should be matched too.
- Ground truth questionable: yes: one painting Insert (after line 9950) starts one column after the
  key's opening quote, unlike its two sibling inserts (1 byte).
