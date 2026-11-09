# Accuracy analysis: csharp-icsharpcode-avaloniailspy-a-few-formatting-changes-and-use-a-struct-instead-of-tuples

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 4 (4 visible) of 14159 nodes, 14159 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

In ILSpy's `CSharpLanguage.cs`: brace and spacing reformatting (`delegate {`, `: base (` ...,
lines 335 and 488-493), and a named tuple replaced by a struct. The return type
`IEnumerable<(string itemType, string fileName, List<PartialTypeInfo> partialTypes)>` (line 498)
becomes `IEnumerable<ProjectItemInfo>` (after line 500). The value
`(handler.EntryType, fileName, (List<PartialTypeInfo>)null)` (line 506) becomes
`new ProjectItemInfo(handler.EntryType, fileName)` (after line 508).

## Mapping

Four visible mismatches, all from `APTED("qualified_name")`. The reformatting produced none.

- Line 506, 3 visible: the tuple's opening `(`, its first `,` and its closing `)`. The ground
  truth keeps them as the `(`, `,`, `)` of the new constructor call's argument list, so only
  `new ProjectItemInfo` and the dropped third element read as edits. Omnidiff deletes them along
  with the `tuple_expression`, because the argument list is under a new `object_creation_expression`
  and APTED does not pair punctuation across unrelated parents. The same pattern appears in
  `go-cloudflare-cfssl-change-zero-value-to-is-zero`. `matcher`.
- Line 498, 1 visible: `PartialTypeInfo`, the generic argument of the tuple's third element. The
  ground truth updates it to the new return type `ProjectItemInfo`. Omnidiff deletes it with the
  tuple type. Which piece of a three-element tuple type "becomes" the struct name has no single
  answer: both `...Info` names are types, and the tuple has no name of its own. `ambiguous`.

## Verdict

- Primary cause: `matcher`
- Other causes: `ambiguous`
- Where a fix would go: the APTED call at `qualified_name` (`solve_syntax_aware_matching.rs`). A
  tuple literal replaced by an object creation should keep its bracket and comma tokens in the
  argument list.
- Ground truth questionable: no
