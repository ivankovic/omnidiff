# Accuracy analysis: scala-com-lihaoyi-mill-new-import-used-to-remove-a-lot-of-code

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 53 (30 visible) of 3579 nodes, 3579 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`ModuleCtx.scala` replaces the hand-written 50-line `upickle.ReadWriter[BufferedValue]` (a visitor
with one `visitX` method per JSON value kind) by a two-line `upickle.Reader.Delegate(
BufferedValue.Builder)` plus an anonymous `new upickle.Writer[BufferedValue] { def write0 ... }`
that keeps the old `write0`. The four `HeaderData` fields change type from `Seq[String]` to
`Located[Seq[Located[String]]]`, and imports change.

## Mapping

Two groups, both inside APTED runs over matched top-level items (reason `APTED("qualified_name")`).

### The split ReadWriter (before lines 52-104, after lines 54-60): 14 visible, 9 invisible

- The ground truth maps `val bufferedRw` -> `val bufferedR`, and inside it the instance expression
  `new upickle.ReadWriter[BufferedValue] {...}` (line 53) -> `new upickle.Reader.Delegate(
  BufferedValue.Builder)` (after line 55): `new`, `upickle`, `.` Identical, `ReadWriter` and the
  `[BufferedValue]` brackets/name updated into `Reader` / `(BufferedValue)`. The old body's
  `{ ... }` (`template_body`, lines 53-104) maps to the body of the new anonymous Writer (after
  lines 57-60), whose head `new upickle.Writer[BufferedValue]` is new.
- omnidiff deletes the val's instance expression, `new upickle.ReadWriter`, and the body's `{`
  (line 53) and `}` (line 104), all `APTED("qualified_name")`. The deleted `[BufferedValue]` is
  then paired with the byte-identical inserted `[BufferedValue]` of the Writer (after line 57) as
  `MovedSubtree`, and `Writer` (after line 57, column 15) is mapped to `core` of
  `upickle.core.ObjVisitor` on before line 82 (MatchButNotIdentical, cross-kind), which is wrong
  on any reading. `matcher`.
- Part of this is a real choice: the old `new upickle.ReadWriter[BufferedValue] {` looks
  textually more like the new `new upickle.Writer[BufferedValue] {` than like
  `new upickle.Reader.Delegate(...)`, so sending the type arguments to the Writer is defensible.
  omnidiff's result is not that alternative made consistent, though: it drops the brackets of the
  body it keeps and pairs `Writer` with an unrelated `core`.

### `Seq[String]` -> `Located[Seq[Located[String]]]`: 16 visible, 14 invisible

Before lines 106-109, after lines 63-66.


- On each of the four field lines the ground truth keeps `Seq[String]` with the inner
  `Seq[Located[String]]` (same `generic_type`, same `type_arguments`, `[` and `]` Identical) and
  treats both `Located[...]` wrappers, including the innermost `[String]` brackets, as inserted.
- omnidiff maps `Seq[String]` to the outer `Located[...]` `generic_type` (MatchButNotIdentical)
  and the whole `[String]` `type_arguments` to the innermost `[String]` (identical subtree, APTED
  then re-tagged `WrapGrowth`). So Seq's own brackets are shown as inserted and the inner
  `Located`'s as kept. Both readings insert the same characters; the ground truth's keeps each
  bracket pair with the type it parameterises. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED in `solve_syntax_aware_matching` (`qualified_name`): keep a
  `type_arguments` container with its `generic_type` rather than preferring a whole identical
  `[String]` subtree deeper down; avoid cross-kind leaf pairs such as `core` -> `Writer`; keep the
  `{`/`}` of a `template_body` whose contents are kept.
- Ground truth questionable: no (the ReadWriter split has a second defensible reading, noted above)
