# Accuracy analysis: csharp-sonarr-sonarr-update-regex

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 19848 nodes, 19848 graded, N:M floor 0 visible |
| Painting, Minimal | 75 bytes wrong of 142358: renderer 75, matcher 0 |
| Painting, Full | 80 bytes wrong of 142358: renderer 80, matcher 0 |
| Ground-truth invariant violations | none |

## The change

One verbatim regex on line 547 is widened: `[_. ](?<multi>multi)[_. ]` becomes `[-_.
\[](?<multi>multi|multilang|multilanguage)[-_. \]]` (extra separator characters and two more
alternatives). Everything else in the 20k-node file is unchanged.

## Painting

Mapping is perfect and both runs per preset are identical with the human mapping, so all of it is
renderer-owned. The regex is one string token, so the painting comes from omnidiff's text diff
inside it.

- Minimal, 75 bytes (22 before + 53 after), Update -> None/Insert: the painting records six pure
  insertions (`-`, `\[`, `|multilang|multilanguage`, `-`, `\`, `]`) and leaves every kept
  character unpainted. omnidiff paints one Update range from the first changed character to the
  last on each side (`_. ](?<multi>multi)[_.` against `-_. \[](?<multi>multi|multilang|multilanguage)[-_. \]`),
  so the unchanged middle, `(?<multi>multi`, and the kept separators are painted as changed and
  the removed side shows a 22-byte Update although nothing was removed. `renderer`.
- Full, 80 bytes (25 + 55): the Full painting interleaves the same insertions with Match entries
  for every kept piece (`[`, `_. `, `]`, `(?<multi>multi`, `)[`, `_. `, `]`), which it paints Move
  because most shifted columns; omnidiff's Full rendering of the string differs over the whole
  changed region, starting at `[`, which it leaves unpainted. `renderer`.

The in-string diff coalesces several small insertions separated by short kept runs into one Update,
where the paintings keep the kept runs as anchors.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the text diff inside a string token (`intra_node_update_ranges` in
  src/diff/text.rs), which merges insertions separated by short kept runs into one Update
- Ground truth questionable: no
