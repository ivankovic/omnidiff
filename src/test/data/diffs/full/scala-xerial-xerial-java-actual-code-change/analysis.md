# Accuracy analysis: scala-xerial-xerial-java-actual-code-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 4 (3 visible) of 2233 nodes, 2233 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In the sbt build `XerialBuild.scala`, `scalacOptions ++= Seq("-encoding", "UTF-8", ...)` (line 60)
is replaced by two `javacOptions` settings built from a new `defaultJavacOptions` val (after lines
44, 62-63), `autoScalaLibrary := false` is added, a three-line XML element `<scala.version>` in
`pomExtra` is joined onto one line, and a dependency line is commented out.

## Mapping

### The replaced options list (before line 60): 2 visible

The ground truth maps the old `Seq(...)` call to both new `Seq(...)` calls on after lines 62-63
through any-one-to-one groups: the 6 old strings to the 7 new strings (MatchButNotIdentical) and
the 5 old commas to the 5 new commas (Identical). omnidiff realises 5 of the 6 string pairs and 4
of the 5 comma pairs (for example `"-encoding"` -> `"-windowtitle"`, which counts). The detail
output does not say which string and which comma it left out of the group, or with what reason.
`matcher`.

Note on the ground truth: after line 44's new
`defaultJavacOptions = Seq("-encoding", "UTF-8", ...)` holds byte-identical copies of the first
two old strings, and the ground truth marks that whole val as inserted. Mapping the old strings
there instead would also be defensible; this is a judgement, not an inconsistency.

### The joined XML element (before lines 75-77): 1 visible, 1 invisible

tree-sitter-scala has no XML-literal support, so `<scala.version>{SCALA_VERSION}</scala.version>`
parses as operator expressions with `ERROR` nodes (the ground truth has a 2:1 group of `ERROR`
nodes here). Joining the three lines changes that error-laden shape. omnidiff deletes the
`postfix_expression` `<scala.version>` (line 75, invisible) and the `.` of `</scala.version>`
(line 77, column 18), reason `APTED("qualified_name")`; the ground truth keeps both. `parse-error`.

## Verdict

- Primary cause: `matcher`
- Other causes: `parse-error`
- Where a fix would go: APTED within the matched `buildSettings` val (`qualified_name`) for the
  options lists; the XML part depends on the grammar's lack of XML literals.
- Ground truth questionable: no
