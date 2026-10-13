# Accuracy analysis: swift-apple-swift-argument-parser-remove-trimming-lines-function-call

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 3371 nodes, 3371 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `Tests/ArgumentParserPackageManagerTests/HelpTests.swift`, the commit removes `.trimmingLines()`
from about fifteen expressions. It replaces
`#if !os(Windows) && !os(WASI) / unsetenv("COLUMNS") / #endif` (before lines 19-21) with
`Platform.Environment[.columns] = nil` (after line 19). The `.trimmingLines()` removals produce no
mismatches.

## Mapping

- **Call parentheses not paired with subscript brackets (2 visible), `matcher`.** The ground truth
  maps `unsetenv("COLUMNS")` (line 20) to the subscript `Platform.Environment[.columns]`, which
  tree-sitter-swift also parses as a `call_expression` with `value_arguments`. It pairs the
  delimiters across kinds: `(` -> `[` and `)` -> `]` (MatchButNotIdentical). omnidiff matches the
  surrounding `call_expression`/`value_arguments` and the argument as the ground truth does, but
  deletes `(`/`)` and inserts `[`/`]` (`APTED("qualified_name")`, op Delete). The tokens are
  different anonymous node kinds, which APTED does not rename. The ground truth updates delimiters
  that keep their role elsewhere too, for example quote changes.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: a post-APTED step that pairs the unmatched opening/closing delimiters of
  an already matched parent (`value_arguments`) with each other, whatever the kind of bracket
- Ground truth questionable: no
