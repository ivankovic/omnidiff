# Accuracy analysis: cpp-tensorflow-tensorflow-new-to-make-unique

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 6 (4 visible) of 231 nodes, 231 graded, N:M floor 0 visible |
| Painting, Minimal | 5 bytes wrong of 2275: renderer 1, matcher 4 |
| Painting, Full | 5 bytes wrong of 2275: renderer 1, matcher 4 |
| Ground-truth invariant violations | none |

## The change

`std::unique_ptr<NnApi> nnapi(new NnApi);` becomes
`std::unique_ptr<NnApi> nnapi = std::make_unique<NnApi>();` (line 28).

## Mapping

- **Parentheses reused (4 visible, 2 invisible, `matcher`, `APTED("qualified_name")`).** The ground
  truth deletes the declarator's initializer list `(new NnApi)` with its `(` and `)`, and it inserts
  the `()` of the new `std::make_unique<NnApi>()` call. Omnidiff maps the before `argument_list` and
  its parens onto the call's `()` (`MatchButNotIdentical`). The two lists belong to different
  constructs: a constructor-style initializer and a call's arguments.

## Painting

- **Matcher (4 bytes in each preset).** The old `(` / `)` and the new `()` are left unpainted
  (Minimal) or painted Move (Full), where the paintings delete and insert them.
- **Renderer (1 byte in each preset).** These runs stay with the human mapping. Minimal paints the
  space after the inserted `=`, which the Minimal painting leaves out. Full leaves the space before
  `=` unpainted, which the Full painting inserts.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: APTED in the `qualified_name` call. An initializer argument list should not
  pair with the argument list of a new call in a different construct.
- Ground truth questionable: no
