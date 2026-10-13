# Accuracy analysis: cpp-laydbird-change-function-signature

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 54 (33 visible) of 7121 nodes, 7121 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

The type `Optional<Variant<GC::Root<DOM::Element>, GC::Root<DOM::Document>>>` in the constructor
signature (line 87) and the return type
`Variant<GC::Root<DOM::Element>, GC::Root<DOM::Document>, Empty>` of `root()` (line 194) are both
replaced by `NullableIntersectionObserverRoot`. On line 96,
`root.has_value() ? root->visit(...) : nullptr` becomes
`root.has<Empty>() ? nullptr : root.visit(...)` with three typed lambdas:
`GC::Root<DOM::Element> const&`, `GC::Root<DOM::Document> const&` and `Empty`.

## Mapping

- **Type fragments moved into the lambdas (32 visible, 20 invisible, `matcher`, `MovedSubtree`).**
  The ground truth deletes the old type spellings and makes the lambda parameter types on line 96
  new. Omnidiff moves `GC::Root<DOM::Document>` out of the constructor signature (line 87, column
  143) into the second lambda's parameter (line 96, column 140). It moves `GC::Root<DOM::Element>`
  out of `root()`'s return type (line 194, column 9) into the first lambda's parameter (line 96,
  column 58). These are byte-identical, but they are fragments of removed declarations in other
  functions. Line 194 is about 100 lines from line 96. A reader shown them as moves would be misled.
- **`nullptr` (1 visible, 1 invisible, `matcher`, `APTED("qualified_name")`).** The ground truth
  maps the before `: nullptr` arm to the `return nullptr;` of the new `[](Empty)` lambda
  (`identical_with_children`), and it makes the after `? nullptr` new. Omnidiff deletes the before
  `nullptr` and matches neither after copy. Mapping it to `? nullptr` would also be defensible.
  Deleting it is not.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the `MovedSubtree` acceptance for small type fragments that leave deleted
  declarations for a different function.
- Ground truth questionable: no
