# Accuracy analysis: java-defects4j-cli-36-optiongroup

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1310 nodes, 1310 graded, N:M floor 0 visible |
| Painting, Minimal | 62 bytes wrong of 9464: renderer 62, matcher 0 |
| Painting, Full | 34 bytes wrong of 9464: renderer 34, matcher 0 |
| Ground-truth invariant violations | 2 |

## The change

Defects4J Cli-36 in `OptionGroup`: `optionMap` becomes a `LinkedHashMap` so groups keep insertion
order (line 37), and the import `java.util.HashMap` (before line 22) becomes
`java.util.LinkedHashMap`, re-sorted below `java.util.Iterator` (after line 23).

## Painting

No mapping mismatches. Every painting run also appears with the human mapping, so all are the
renderer's.

- Minimal 56 / Full 34 bytes, None -> Move: the import (before line 22, after line 23). The
  paintings pair it as a moved line (`import java.util.` and `HashMap;` matched, `Linked`
  inserted), since it moved past `import java.util.Iterator;`. omnidiff paints neither the
  import nor `Iterator` as moved, so the reorder is not shown at all (`move_attribution.md`
  discusses the two accounts of such a reorder; here neither is painted). Under Full only the
  kept prefix `import java.util.` differs, the identifier is painted as an Update in both.
  `renderer`.
- Minimal, 6 bytes, Update -> Insert: `Linked` of `new LinkedHashMap` (after line 37). omnidiff
  narrows `HashMap` -> `LinkedHashMap` and labels the added prefix Update; the painting labels it
  Insert. `renderer`.

`invariants.txt` reports 2 violations of invariant 16 on the import ("paints `HashMap` against
`LinkedHashMap` wrongly for a Minimal reading"). The Minimal painting's entries there are a
narrowed reading (`HashMap;` -> `HashMap;` and an inserted `Linked`), so the evidence does not
show what the check objects to; the disagreement with omnidiff on those rows is the missing `Move`
above.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` move attribution (`crossed_backwards` / `reconcile_moves`) for
  a re-sorted import that is also updated; the label of a narrowed Update's added prefix
- Ground truth questionable: no (invariant 16 fires on the import, but the cause is not visible
  in the painting's entries)
