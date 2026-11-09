# Accuracy analysis: javascript-microsoft-typescript-broken-js-remove-string-fragment

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 464 (320 visible) of 29653 nodes, 29653 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

TypeScript compiler baseline (`tests/baselines/reference/tsserver/skipLibCheck/
jsonly-inferred-project.js`), a tsserver log, not valid JavaScript. The commit removes the line
`/// <reference no-default-lib="true"/>` (before line 21) and the same fragment from three string
literals (before lines 67, 428, 698). Everything else is unchanged, shifted up one line.

## Mapping

All 464 mismatches (320 visible, 144 invisible) are four crossed pairs of twin subtrees, 58 nodes
per line, on before lines 756-759 and 826-829. The log lines

```
Info seq  [hh:mm:ss:mss] DirectoryWatcher:: Added:: WatchInfo: ...
Info seq  [hh:mm:ss:mss] Elapsed:: *ms DirectoryWatcher:: Added:: WatchInfo: ...
```

parse as `labeled_statement`s with `ERROR` nodes inside, and the tail starting at
`DirectoryWatcher::` is a byte-identical subtree on both lines (col 26 on the first, col 40 on the
second). The ground truth maps each line to itself (before line 756 to after line 755, and so on).
omnidiff crosses them: before 756 col 26 -> after 756 col 40, before 757 col 40 -> after 755 col
26, and likewise for 758/759 (`node_modules`) and the `Close::` lines 826/827 and 828/829. The 8
roots are paired by `IdenticalHash`, the 456 descendants follow by `IdenticalHashOfAncestor`. The
choice between the two identical candidates picks the twin one line further down, where the
untouched position (same line, same column) was available. `matcher`.

The twins exist only because a log is parsed as JavaScript (the fixture's test note: error
recovery is unstable), but the trees on both sides have the same shape here; the wrong choice is
the twin selection, not a parse divergence. `parse-error` is context, not the cause.

## Verdict

- Primary cause: `matcher`
- Other causes: `parse-error`
- Where a fix would go: twin selection in `IdenticalHash` (prefer the candidate at the same
  relative position / same row context when several identical subtrees compete)
- Ground truth questionable: no
