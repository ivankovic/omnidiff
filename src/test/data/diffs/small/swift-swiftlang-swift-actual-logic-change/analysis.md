# Accuracy analysis: swift-swiftlang-swift-actual-logic-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 36 (19 visible) of 6688 nodes, 6688 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `utils/swift-dev-utils/Sources/CrashReduce/CrashLog.swift`, `findFrameSignature(...) -> String?`
becomes `findInterestingSymbols(...) -> [String]`. The final `return symbols.first` (before line 96)
grows into a `guard let firstSymbol = symbols.first` and a search for a second symbol (after lines
97-114). Its caller (before lines 125-126)

```
sig = Self.findFrameSignature(...).map { Signature(symbol: $0, assertion: assertion) }
```

becomes `let syms = Self.findInterestingSymbols(...)` plus
`if !syms.isEmpty { sig = Signature(symbols: syms, ...) }` (after lines 143-146).

## Mapping

All mismatches are from the APTED run of the qualified-name pass (`APTED("qualified_name")`) or
print no reason.

- **`return symbols.first` paired by position (8 visible, 6 invisible), `matcher`.** The ground
  truth moves `symbols.first` into the new `guard let firstSymbol = symbols.first` (after line 97)
  and keeps the `return` keyword as the final `return [firstSymbol]` (after line
  114). omnidiff pairs the whole statement with
  `return [firstSymbol, next.short!.symbol]` (after line 111): `symbols` -> `next`, `first` ->
  `short` (Update), `.` -> `.`. Tree edit distance renames in place rather than moving the
  identical `symbols.first` to a new statement.
- **Caller split into declaration plus guarded assignment deleted (10 visible, 10 invisible),
  `matcher`.** The ground truth spreads before lines 125-126 over the new statements:
  - `Self.findFrameSignature` -> `Self.findInterestingSymbols` (update) in `let syms = ...`
  - `sig` and `Signature(` ... `)` -> the assignment inside the new `if`
  - `$0` -> `syms`

  omnidiff deletes all of it (op Delete). The old statement now spans two statements, one of them
  nested a level deeper inside an `if`. APTED cannot express this, so it deletes.
- **`return recurse.min()` (1 visible, 1 invisible), `matcher`.** The ground truth puts before line
  94 in a 1:2 multi-map group with the new `guard let min = recurse.min() else { return [] }` /
  `return [min]` (floor 0, so a one-to-one pairing can satisfy it). omnidiff pairs nothing in the
  group; the reason is not printed.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: `APTED("qualified_name")` (src/diff/solve_syntax_aware_matching.rs) and a
  post-APTED step that re-pairs deleted statements whose identical sub-expressions (`symbols.first`,
  `Self.findFrameSignature(...)`'s arguments) landed in inserted statements
- Ground truth questionable: no
