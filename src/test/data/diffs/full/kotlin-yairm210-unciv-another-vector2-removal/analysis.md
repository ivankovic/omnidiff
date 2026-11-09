# Accuracy analysis: kotlin-yairm210-unciv-another-vector2-removal

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 12 (9 visible) of 12719 nodes, 12719 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`GameInfo.kt`: eight `import com.unciv.logic.civilization.X` lines (before lines 16-23) collapse
into `import com.unciv.logic.civilization.*`, `import java.util.UUID` becomes
`import java.util.*`, two imports are removed, and one `tile.position.toVector2()` argument
becomes `tile.position`.

## Mapping

- **Eight imports to one wildcard** (9 visible, 2 invisible, reason `APTED("fast_fallback")`). The
  ground truth maps the first, `import com.unciv.logic.civilization.Civilization` (line 16), to
  the wildcard import: `import`, `com`, `unciv`, `logic`, `civilization` and the dots kept,
  `Civilization` -> `*` updated, the other seven deleted. Omnidiff maps none of the eight to the
  wildcard: all eight are deleted and the wildcard inserted. `solve_import_path_similarity` refuses
  a pair when the after import has rivals (`import_paths_have_no_rival`), and here seven other
  before imports share the same path prefix, so the import is left to the fallback, which does not
  pair it either. Choosing which of eight rivals became the wildcard has no unique answer; the
  ground truth's "first one" is reasonable, and so is delete-all plus insert. `ambiguous`.
- **`java.util.UUID` -> `java.util.*`** (1 invisible, reason `APTED("import_path_similarity")`): the
  import statement is paired, but its `qualified_identifier` is left unmapped where the ground
  truth maps it. Invisible only. `matcher`.

## Verdict

- Primary cause: `ambiguous`
- Other causes: `matcher`
- Where a fix would go: none for the 8:1 collapse (by design: the import pass refuses to pick
  between rivals). The unmapped `qualified_identifier` inside a paired import is in the APTED
  call of `solve_import_path_similarity` (src/diff/solve_syntax_aware_matching.rs).
- Ground truth questionable: no
