# Accuracy analysis: cpp-qpdf-qpdf-move-to-assert-from-if-and-throw

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 14 (9 visible) of 2569 nodes, 2569 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `InputSource::findFirst`, the guard
`if ((strlen(start_chars) < 1) || (strlen(start_chars) > size)) { throw std::logic_error("..."); }`
(lines 76-79) becomes
`util::assertion(!(strlen(start_chars) < 1 || strlen(start_chars) > size), "..." //);` (after lines
79-82). An include and `using namespace qpdf;` are added, and an unreachable `throw` is removed
(line 158).

## Mapping

- **Call frame deleted (6 visible, 3 invisible, `matcher`, `APTED("qualified_name")`).** The ground
  truth maps the thrown `std::logic_error(...)` call to `util::assertion(...)`: `std` -> `util` and
  `logic_error` -> `assertion` are Updates, and `::`, `(`, `)` and `;` are kept. The message string
  is kept as the call's argument. Omnidiff deletes the frame (lines 77-78) and inserts the new one.
  The call's parent changes from a `throw_statement` inside an `if` block to an
  `expression_statement`, and APTED does not carry it across.
- **Condition operator (3 visible, 2 invisible, `matcher`, `APTED("qualified_name")`).** The ground
  truth maps the `||` `binary_expression` of the `if` condition (line 76) into `!(A || B)` (line
  80). Its two parenthesized operands `(A)` and `(B)` form a 2:1 group with the one new `( ... )`,
  and one pair of them is expected. Omnidiff deletes `||` and both parenthesized operands. A
  one-to-one mapping can satisfy the group, so this is not N:M floor.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED in the `qualified_name` call. It should carry a call and a condition
  across when an `if`/`throw` is rewritten into an assertion call and the parent kinds change.
- Ground truth questionable: no
