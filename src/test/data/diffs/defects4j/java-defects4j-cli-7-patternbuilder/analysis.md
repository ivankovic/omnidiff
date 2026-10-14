# Accuracy analysis: java-defects4j-cli-7-patternbuilder

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 2292 nodes, 2292 graded, N:M floor 0 visible |
| Painting, Minimal | 62 bytes wrong of 11896: renderer 62, matcher 0 |
| Painting, Full | 34 bytes wrong of 11896: renderer 34, matcher 0 |
| Ground-truth invariant violations | 2 |

## The change

Cli-7: `PatternBuilder` keeps its options in a `LinkedHashSet` instead of a `HashSet`, so the
order is kept. The import `java.util.HashSet` becomes `java.util.LinkedHashSet` and, being
re-sorted, now sits below `import java.util.Iterator;` (before line 19, after line 20); line 67
changes `new HashSet()` to `new LinkedHashSet()`.

## Painting

The tree mapping matches the ground truth exactly; every painting disagreement survives with the
human mapping, so all of it is the renderer's (matcher 0 bytes on both presets).

- Minimal and Full, the moved import (before line 19, after line 20): both paintings mark the
  import as moved (`import java.util.` Match to the row below, identical, so Move). omnidiff
  paints no move here: the import's text outside the renamed identifier stays unpainted.
  Minimal: 25 bytes before + 31 after (these runs also cover the identifier, see below); Full:
  17 + 17 bytes, None -> Move. The two adjacent imports swapped order; the painter attributes the
  move to the renamed import, and omnidiff paints neither import as moved. `renderer`.
- Minimal, `Linked` (after lines 20 and 67), Update -> Insert, 6 bytes on line 67 (line 20's
  falls inside the run above): omnidiff narrows the identifier update to the added word but labels
  it Update; the Minimal painting calls the added word an Insert and leaves `HashSet` as kept
  (Move on line 20, unpainted on line 67). Full paints the whole identifier as a Match (Update),
  which omnidiff matches. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the text renderer (`src/diff/text.rs`): move painting for an updated node
  whose position among its siblings changed, and the label of a narrowed Update whose before side
  narrows to nothing.
- Ground truth questionable: yes, mildly: invariants.txt flags the Minimal painting twice (rule
  16) for painting `HashSet` against `LinkedHashSet` on the import line instead of leaving the
  shared part alone; the painter painted it Move because the line moved.
