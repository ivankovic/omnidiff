# Accuracy analysis: java-defects4j-cli-37-defaultparser

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 9 (6 visible) of 6188 nodes, 6188 graded, N:M floor 0 visible |
| Painting, Minimal | 99 bytes wrong of 40900: renderer 4, matcher 96 |
| Painting, Full | 42 bytes wrong of 40900: renderer 29, matcher 40 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-37 in `DefaultParser.isShortOption`: the one-line `return token.startsWith("-") &&
token.length() >= 2 && options.hasShortOption(token.substring(1, 2));` (before line 302) is split
into a guard `if (!token.startsWith("-") || token.length() == 1) { return false; }` (after lines
302-305), the extraction of `optName` up to `=`, and `return options.hasShortOption(optName);`
(after line 310). The comment `// remove leading "-" and "=value"` stays between them.

## Mapping

All through `APTED("large_flat_subtree")`, including the after-side `;` "descendant" line.

- **The negated condition's operators (3 visible, 2 invisible).** The ground truth reads `a && b`
  -> `!a || b'` as operator Updates: `&&` -> `||`, `>=` -> `==`, `2` -> `1` (before line 302,
  after line 302), and both paintings paint them as Updates. omnidiff matches the operands but
  leaves the `binary_expression` containers and their operators unmatched, so the operators are
  deleted and inserted. `matcher`.
- **Which `return` the old one became (3 visible, 1 invisible).** The ground truth maps the old
  `return` and `;` to `return false;` in the guard (after line 304). omnidiff deletes `return`
  and pairs the `;` with `return options.hasShortOption(optName);` (after line 310), where the
  old statement's `options.hasShortOption(...)` operand also goes in both mappings. Either
  `return` can be called the old one. `ambiguous`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began printing them.

## Painting

- Minimal 68 bytes, Move -> None (absent with the human mapping): the comment `// remove leading
  "-" and "=value"` (before line 304, after line 307). omnidiff's `;` pair from line 302 to line
  310 jumps over it, and the walk-order move attribution paints the comment `Move` instead of the
  `;` (`move_attribution.md`). Nothing in the comment moved. `renderer` (exposed by the
  `ambiguous` pairing).
- Matcher, Minimal 13 / Full 18 bytes, Delete/Insert/None -> Update/Move: the operators `&&`,
  `>= 2`, `||`, `== 1` on line 302, painted Delete/Insert where the paintings paint Update.
- Ambiguous, Minimal 15 / Full 16 bytes: `return` (before 302) painted Delete and `return ` / `;`
  (after 304) painted Insert where the paintings paint `Move`; the `;` of after line 310 painted
  `Move`/unpainted where the paintings insert it.
- Renderer (also with the human mapping), Minimal 3 / Full 8 bytes: `{` and `}` of the new guard
  block (after lines 303, 305) and the kept `;` (before 302), and under Full the indentation of
  after line 302. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`, `ambiguous`
- Where a fix would go: `diff::text` move attribution (`crossed_backwards` / `reconcile_moves`),
  which paints a comment for a `;` that jumped over it; `APTED("large_flat_subtree")` for the
  operator containers of a negated condition
- Ground truth questionable: no
