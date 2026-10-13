# Accuracy analysis: go-grafana-grafana-interesting-how-escapes-impact-string-parsing

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 99 (81 visible) of 67446 nodes, 67446 graded, N:M floor 0 visible |
| Painting, Minimal | 1401 bytes wrong of 245289: renderer 129, matcher 1400 |
| Painting, Full | 1627 bytes wrong of 245289: renderer 1292, matcher 1535 |
| Ground-truth invariant violations | 1 |

## The change

In a generated OpenAPI file, the `"action"` property of `schema_..._JobSpec` gets two new enum
values, `deleteResources` and `releaseResources`. Each one adds a `\n - \`\"...\"\` ...` entry
to the long `Description` string on line 1460, and a string to the `Enum` list on line 1464.

## Mapping

All 99 mismatches (81 visible, 18 invisible) are before-side nodes that omnidiff deletes, all
through `APTED("large_flat_subtree")`. Their after counterparts are painted Insert. They cover:
the `Properties: map[string]spec.Schema{` literal_value (line 1457) with its closing `}` (line
1516) and the commas after every entry (lines 1466-1515); the `"action"` keyed element and its
`SchemaProps: spec.SchemaProps{` path (lines 1458-1459); every leaf of the `Description` string
(line 1460); the `,` of lines 1461-1463; and `Enum`, `[]interface{}{`, the commas and the braces
of line 1464.

Why: `solve_large_flat_subtrees.rs` diffs the largest flat container of each top-level item first
(`largest_flat_container_in`). It picks that container on each side independently. A Go
interpreted string is a flat node: `"`, each text fragment and each escape sequence are separate
children. This is the "escapes impact string parsing" of the fixture's name. Counting fragments
and escapes in the text, the before `Description` string has about 45 children and the after one
about 57. `FLAT_MIN_CHILDREN` is 50 (`apted/common/myers.rs`). So the after side takes the string.
The before side falls back to `widest_data_literal_container` and takes the widest
`literal_value`, which fits the deleted nodes: the `Properties` map. `apted::for_nodes` is then
called on a map against a string. That pair cannot correspond, and the nodes under it are fixed
as deleted before the container pass runs. The other property entries (`"repository"`, `"pr"`,
...) are matched later. Their commas and the map's braces are not. `matcher`.

The child counts are estimated from the source text, not read from the parser.

## Painting

There is one painting ("Only one solution"). It inserts the two new string entries and the two
new enum strings, and nothing else.

- Matcher, Minimal 1401 bytes (`matcher_bytes` 1400), `Delete/Insert -> None`. The whole
  `Description` line, `SchemaProps` and the `Enum` line are painted Delete and Insert on the two
  sides. In Full the same comes to about 1515 bytes, plus the commas and braces of the map (lines
  1457-1516).
- Renderer, Full about 112 bytes, `Move -> None`. On line 1464, the unchanged enum strings after
  the inserted `"deleteResources",` are painted Move (shifted on the same row). With the human
  mapping, Full also paints the shifted tail of the `Description` string Move (585 + 587 bytes).
  With the human mapping, Minimal paints 63 bytes per side of the repeated `\n`, ` - \``, `\"`
  leaves Move. Omnidiff's own Delete/Insert hides both of these. Why the human mapping makes
  those repeated leaves Move in Minimal was not traced. `renderer`.
- Ground truth, 2 bytes with the human mapping, `Insert -> None`: invariant [11]. The painting
  leaves the `\n` escape at after line 1460, columns 936-938, unpainted, although the tree
  inserts it. `ground-truth`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`, `ground-truth`
- Where a fix would go: `largest_flat_container_in` / `solve` in `solve_large_flat_subtrees.rs`.
  Pick the flat pair consistently on both sides (same kind, corresponding position), or skip
  the pre-match when the two picks are not the same kind.
- Ground truth questionable: yes, minor. Invariant [11] reports an inserted `\n` that the
  painting leaves unpainted.
