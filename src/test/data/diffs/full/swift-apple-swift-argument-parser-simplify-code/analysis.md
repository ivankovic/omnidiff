# Accuracy analysis: swift-apple-swift-argument-parser-simplify-code

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 3817 nodes, 3817 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `Sources/ArgumentParser/Usage/DumpHelpGenerator.swift`, two `import class Foundation.JSONEncoder`
lines go. A five-statement body:

```
let encoder = JSONEncoder()
encoder.outputFormatting = ...
encoder.outputFormatting.insert(...)
guard let encoded = try? encoder.encode(self.toolInfo) else { return "" }
return String(...)
```

(lines 32-36) collapses to one expression, `JSONEncoder.encode(self.toolInfo)` (after line 30).

## Mapping

- **Which `JSONEncoder`/`encoder` feeds the new receiver (2 visible), `ambiguous`.** The ground
  truth takes the receiver `JSONEncoder` from the identical identifier in `let encoder =
  JSONEncoder()` (line 32), and the rest of the call, `.encode(self.toolInfo)`, from line 35. It
  deletes line 35's `encoder`. omnidiff (`APTED("qualified_name")`) keeps the whole
  `encoder.encode(self.toolInfo)` of line 35 together and renames `encoder` -> `JSONEncoder`
  (Update). It deletes line 32's `JSONEncoder`. The ground truth follows identical text across
  statements, and omnidiff keeps one expression intact with a rename. Each shows one identifier
  change, so both are defensible readings of the collapse.

## Verdict

- Primary cause: `ambiguous`
- Other causes: none
- Where a fix would go: none: ambiguous (would need a preference for identical text over
  structural coherence in `APTED("qualified_name")`)
- Ground truth questionable: no
