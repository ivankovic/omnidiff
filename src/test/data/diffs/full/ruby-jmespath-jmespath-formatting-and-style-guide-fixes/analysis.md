# Accuracy analysis: ruby-jmespath-jmespath-formatting-and-style-guide-fixes

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 17 (15 visible) of 4586 nodes, 4523 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`lib/jmespath/parser.rb` style rewrite between equivalent Ruby forms: statement modifiers
(`x while c`, `x if c`) become block forms, `loop do ... break unless c end` becomes
`begin ... end while c`, `case/when` becomes `if/elsif`, `%i[a b]` becomes `[:a, :b]`,
`left.is_a?(X)` becomes `X === left`, `key: v` becomes `:key => v`, plus re-indentation.
description.md notes that a perfect answer needs a 2:1 map (bare symbol and its text to one
`simple_symbol`); that case does not show in the mismatches (N:M floor 0).

## Mapping

All 17 mismatches are before-side nodes that omnidiff leaves unmapped (Delete, reason
`APTED("qualified_name")`, the APTED call per matched method). Each is at a place where the
syntax kind of the parent changes between the two forms, and the ground truth keeps the shared
keyword or operand:

- Statement modifier to block form: `while` (line 63, `while_modifier` -> `while`), `if` on lines
  115 and 296 (`if_modifier` -> `if`). 3 visible.
- `loop do ... end` to `begin ... end while`: the closing `end` of the `do_block` on lines 117,
  257 and 299 (-> `end` of `begin`), the `end` of `case` on line 254 (-> `end` of the new `if`),
  and on lines 294-297 the inner `if ... end` with its `if`, `then` and `end`. 7 visible,
  2 invisible.
- `%i[` and `]` of a `symbol_array` -> `[` and `]` of an `array` on lines 110 and 196. 4 visible.
- `left` on line 208, which moves from receiver (`left.is_a?(...)`) to right operand
  (`... === left`). 1 visible.
- `disable_visit_errors` on line 219, `hash_key_symbol` -> `simple_symbol`. 1 visible.

Omnidiff renders these keywords and brackets as deleted and inserted, though they are kept. The
ground truth's reading is the reasonable one. Which after node took each keyword's place (if
any) is not printed in the evidence; the common factor is the change of container kind, which
APTED prices as delete plus insert of the container, and the keyword leaves go with it.
`matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED call in `solve_qualified_name_groups`
  (src/diff/solve_syntax_aware_matching.rs), or a post-pass that pairs identical keyword leaves
  (`end`, `if`, `while`) and bracket tokens of containers whose kind changed between equivalent
  Ruby forms.
- Ground truth questionable: no
