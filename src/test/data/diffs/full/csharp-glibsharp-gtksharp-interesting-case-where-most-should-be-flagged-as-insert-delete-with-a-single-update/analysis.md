# Accuracy analysis: csharp-glibsharp-gtksharp-interesting-case-where-most-should-be-flagged-as-insert-delete-with-a-single-update

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 2468 nodes, 2468 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

Generated GtkSharp bindings. The nested `class FinalizerInfo` and the `~FileAttributeMatcher()`
finalizer (before lines 118-138) are replaced by
`protected override Action<IntPtr> DisposeUnmanagedFunc { get { return
g_file_attribute_matcher_unref; } }`
(after lines 119-123). The fixture's name says most of it should read as delete/insert, with a
single carried-over part.

## Mapping

One visible mismatch. The ground truth maps the identifier `g_file_attribute_matcher_unref` in
`g_file_attribute_matcher_unref (handle);` (before line 129, inside `FinalizerInfo.Handler`) to
the one in `return g_file_attribute_matcher_unref;` (after line 121). That is the single thing
it carries from the old code into the new code. Omnidiff deletes it through
`APTED("qualified_name")`. The two sit under unrelated parents (a call statement in a method of a
deleted nested class, and a return in a new property getter). APTED does not move a leaf between
unrelated parents, and no later pass picks up a lone identical identifier. The third copy
(line 113, in `Unref`) is unchanged and matched in both. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: leaf move detection after APTED at the `qualified_name` call site, for
  an identical identifier carried from a deleted construct into the construct that replaces it.
- Ground truth questionable: no
