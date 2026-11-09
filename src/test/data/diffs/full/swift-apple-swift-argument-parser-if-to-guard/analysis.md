# Accuracy analysis: swift-apple-swift-argument-parser-if-to-guard

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 5 (5 visible) of 4101 nodes, 4101 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `Sources/ArgumentParser/Parsable Types/ParsableArguments.swift`, the commit drops a
`#if compiler(>=6.0)` import block. It rewrites

```
if let optionsRange = name.range(of: "_options"), optionsRange.upperBound == name.endIndex
{ return String(name[..<optionsRange.lowerBound]) } else { return name }
```

(lines 44-50) as `guard let matchRange = name.firstMatch(...), matchRange.end == name.endIndex
else { return name }` followed by `return String(name[..<matchRange.start])` (after lines 38-45).

## Mapping

- **Guard braces taken from the `then` block instead of the `else` block (4 visible),
  `matcher`.** The ground truth maps `if_statement` -> `guard_statement` and follows the body that
  survives: the `else` keyword (line 48), the `else` block's `{` (line 48) and its `}` (line 50) map
  to the guard's `else { ... }` (after lines 41-43). The `then` block's `{` (line 46) is deleted.
  omnidiff (`APTED("qualified_name")`) pairs the braces by position. The first `{` (line 46) goes
  to the guard's `{`, and `else`, the second `{` and the final `}` are deleted. The guard's body
  `return name` is matched from the `else` block, so omnidiff's brace pairing does not agree with
  its own body pairing. A reader sees `else {` and `}` deleted and re-inserted around an unchanged
  `return name`.
- **Renamed binding deleted (1 visible), `matcher`.** The ground truth updates `optionsRange`
  (line 44) to `matchRange` (after line 39). omnidiff leaves it unmatched (op Delete,
  `UnresolvedNode`): nothing paired the `if let` binding with the `guard let` binding.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: `APTED("qualified_name")` (src/diff/solve_syntax_aware_matching.rs):
  bracket tokens should follow the block whose contents they enclose (bracket pairs share fate
  with their body). The `if let` -> `guard let` binding needs a positional rename after APTED.
- Ground truth questionable: no
