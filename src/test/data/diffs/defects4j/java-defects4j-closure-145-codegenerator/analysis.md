# Accuracy analysis: java-defects4j-closure-145-codegenerator

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 15329 nodes, 15329 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 65421: renderer 0, matcher 0 |
| Painting, Full | 1 bytes wrong of 65421: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-145: `CodeGenerator.isOneExactlyFunctionOrDo` looks through labels: the body is wrapped
in a new `if (n.getType() == Token.LABEL) { ... } else { ... }` and the old statements gain new
branches around them (after lines 709-727).

## Painting

Mapping is exact and Minimal matches its painting. The single Full byte is renderer-owned (it
survives with the human mapping).

- Full, Insert -> None, 1 byte: the trailing space after `{` on the inserted after line 717
  (`if (getNonEmptyChildCount(n, 2) == 1) { `). Full paints the inserted line through to its end;
  the Full painting stops at the `{`. `renderer` (trailing whitespace at the end of an inserted
  range).

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: trailing-whitespace handling of Full ranges in `src/diff/text.rs`
  (`ranges_for_options` trims only for Minimal).
- Ground truth questionable: no.
