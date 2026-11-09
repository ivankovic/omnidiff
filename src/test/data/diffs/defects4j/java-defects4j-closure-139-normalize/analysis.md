# Accuracy analysis: java-defects4j-closure-139-normalize

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 4 (2 visible) of 7195 nodes, 7195 graded, N:M floor 0 visible |
| Painting, Minimal | 90 bytes wrong of 42560: renderer 84, matcher 6 |
| Painting, Full | 174 bytes wrong of 42560: renderer 168, matcher 16 |
| Ground-truth invariant violations | none |

## The change

Closure-139: `Normalize` rewrites non-hoisted function declarations into `var` assignments (new
`normalizeFunctionDeclaration` / `rewriteFunctionDeclaration`, after lines 281-335), and in the
duplicate-declaration handler the old `if (parent.getType() == Token.VAR) { ... }` (before line
521) becomes the `else if` branch of a new `if (v != null && parent.getType() ==
Token.FUNCTION) { ... }` (after 546-553). The comment `// If name is "arguments", Var maybe
null.` that opened the old `if` body now sits above the new code (after 543).

## Mapping

- The old `if` keyword, 2 visible + 2 invisible. The ground truth maps the old `if_statement`
  (before 521) to the `else if` it became (after 553). omnidiff maps the `if_statement` node and
  its `if` keyword to the new outer `if` (after 546; MatchButNotIdentical / Identical,
  `APTED("qualified_name")`), while the condition `(parent.getType() == Token.VAR)` and the body
  still map to the `else if`. The statement is split between the new wrapper and its old
  branch. `matcher`.

## Painting

- Matcher, Minimal 6 / Full 16 bytes: `if ` on after line 546 left unpainted where the painting
  has Insert, and ` if` of `} else if` on after line 553 painted Insert. `matcher`.
- Renderer, Minimal 84 bytes (survive with the human mapping): the comment
  `// If name is "arguments", Var maybe null.` (before 522, after 543), None -> Move. The
  ground truth moves it out of the old `if` body; its relative order is unchanged in the text,
  and omnidiff's move attribution leaves it unpainted. `renderer`.
- Renderer, Full 168 bytes: the same comment with its indentation (96), and the opposite case:
  Full paints `if (parent.getType() == Token.VAR) {` (before 521, after 553, 72 bytes) Move,
  where the painting leaves it unpainted as the kept `else if`. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`
- Where a fix would go: move attribution in `src/diff/text.rs` (which of a reordered pair is the
  mover); the split `if` in APTED("qualified_name"), `src/diff/solve_syntax_aware_matching.rs`.
- Ground truth questionable: no.
