# Accuracy analysis: java-defects4j-gson-11-typeadapters

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 10 (6 visible) of 17943 nodes, 17943 graded, N:M floor 0 visible |
| Painting, Minimal | 30 bytes wrong of 62231: renderer 0, matcher 30 |
| Painting, Full | 42 bytes wrong of 62231: renderer 0, matcher 42 |
| Ground-truth invariant violations | none |

## The change

In the `NUMBER` type adapter's `switch (jsonToken)`, a `case STRING:` label is added (line 371
after) under `case NUMBER:`, so a numeric string also returns a `LazilyParsedNumber`.

## Mapping

tree-sitter-java makes the after `case NUMBER:` its own (empty) `switch_block_statement_group` and
puts `case STRING:` with the `return` in the next one.

- Ground truth: the before group `case NUMBER: return ...;` (line 370) maps to the after group that
  holds the `return` (the label `case NUMBER:` stays put); `case STRING:` (line 371) is new.
- omnidiff (`APTED("large_flat_subtree")`) maps the before group, its `switch_label` and the label's
  `case`, `NUMBER`, `:` to the after `case STRING:` group (`NUMBER` -> `STRING` as an Update)
  because that group keeps the `return` body, and treats the after `case NUMBER:` on line 370 as
  inserted.

6 visible (the label tokens on both sides), 4 invisible (groups and labels). The ground truth's
reading, where an unchanged label stays and a label is added, is the plain one. `matcher`.

## Painting

All matcher-caused (none with the human mapping). Both presets, line 370-371: `Update -> None` on
before `NUMBER` (6 bytes), `Insert -> None` on after line 370 `case NUMBER:` and `None -> Insert` on
after line 371 `case STRING:` (12 + 12 bytes Minimal; 18 + 18 with indentation in Full). Minimal 30
bytes, Full 42. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED fallback (large_flat_subtree): switch label identity versus body when
  a fall-through label is added
- Ground truth questionable: no
