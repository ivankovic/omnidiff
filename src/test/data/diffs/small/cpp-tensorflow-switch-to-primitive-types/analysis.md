# Accuracy analysis: cpp-tensorflow-switch-to-primitive-types

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 6 (6 visible) of 4603 nodes, 4603 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

TensorFlow's own type aliases are replaced by standard ones: `string` becomes `std::string` and
`uint64` becomes `uint64_t` throughout `buf_rendezvous.cc`. One parameter list is rewrapped.

## Mapping

- **Cross-kind type change (6 visible, `matcher`, `APTED("qualified_name")`).** tree-sitter-cpp
  parses `uint64` as a `type_identifier` and `uint64_t` as a `primitive_type`. The ground truth
  matches each pair (`MatchButNotIdentical`): the four `reinterpret_cast<uint64>` on lines 81-84,
  the parameter type on line 159, and the cast on line 267. Omnidiff deletes the before node and
  inserts the after one every time. The `string` -> `std::string` changes, which stay within type
  kinds, are not in the mismatch list. This is the same gap as
  `cpp-libreoffice-fraction-type-to- double` (`Fraction` -> `double`).

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED's label cost. It would let `type_identifier` and `primitive_type` in a
  type slot be updated into each other.
- Ground truth questionable: no
