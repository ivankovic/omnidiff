# Accuracy analysis: java-defects4j-lang-17-charsequencetranslator

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 3 (0 visible) of 1121 nodes, 1121 graded, N:M floor 0 visible |
| Painting, Minimal | 12 bytes wrong of 10076: renderer 0, matcher 12 |
| Painting, Full | 287 bytes wrong of 10076: renderer 287, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Lang-17. In `CharSequenceTranslator.translate(CharSequence, Writer)`, `len` becomes
`input.length()` instead of a code point count. The `consumed == 0` branch now advances `pos` itself
and `continue`s. The `else` wrapper, the inner `if (pos < len - 2)` guard and the `pos--`/`pos++`
corrections are removed. This leaves the `for` loop directly in the `while` body.

## Mapping

Only 3 invisible mismatches; no visible ones. All have reason `APTED("qualified_name")`.

- **Unwrapped call mapped one level off (2 invisible), `matcher`.** On line 83,
  `Character.codePointCount(input, 0, input.length())` becomes `input.length()`. The ground truth
  maps the inner argument `input.length()` to the new initializer and deletes the outer call.
  omnidiff maps the outer `method_invocation` to the new `input.length()` (MatchButNotIdentical),
  and deletes the inner one. The leaves still land where the ground truth puts them, so no visible
  node disagrees. The painting effect is described below.
- **`if (consumed == 0)` deleted (1 invisible), `matcher`.** The ground truth maps the
  `if_statement` on line 86 to the new one (MatchButNotIdentical; its `else` branch is gone).
  omnidiff gives the statement node itself op Delete. Its visible descendants agree with the ground
  truth, so a reader does not see this.

## Painting

- **Matcher, Minimal, 12 bytes (`Move -> None`).** `input.` on line 83 is painted Move on both
  sides. Because of the outer/inner wrapper choice, the matched `input` and `.length()` leaves sit
  under a different parent. The painting leaves them unpainted. The runs disappear when the human
  mapping is rendered. `matcher`.
- **Renderer, Full, 287 bytes (`None -> Move`).** The Full painting marks `for (int pt = 0; pt <
  consumed; pt++) {` (line 93 before, line 94 after) and `pos += Character.charCount(...)` (line 95
  on both sides) as Move: the loop left the `else` block for the `while` body. omnidiff's Full
  rendering leaves both lines unpainted, also with the human mapping (51 + 78 bytes per side). The
  other 29 bytes are whitespace that Full paints: the space before `input.length()` on line 83, the
  indentation of `pos--;` on line 100, and the indentation of the loop's closing `}` on line 96
  after. Minimal leaves the moved loop unpainted, so only Full disagrees. `identical_or_move` in
  `src/diff/text.rs` paints a matched node Move only when its start column changes (or it crosses
  backwards). A row-only shift counts as an unrelated edit above, and this statement kept its
  column. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`
- Where a fix would go: `src/diff/text.rs` (Full: Move painting for a statement re-parented out of a
  removed `else` block); secondary: the `APTED("qualified_name")` choice of wrapper for an unwrapped
  call
- Ground truth questionable: no
