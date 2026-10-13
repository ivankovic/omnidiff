# Accuracy analysis: java-defects4j-cli-9-parser

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 14 (10 visible) of 3475 nodes, 3475 graded, N:M floor 0 visible |
| Painting, Minimal | 36 bytes wrong of 26526: renderer 10, matcher 41 |
| Painting, Full | 40 bytes wrong of 26526: renderer 4, matcher 36 |
| Ground-truth invariant violations | none |

## The change

Cli-9: `Parser.checkRequiredOptions` appends `", "` after each missing option name (new line
320) and strips the trailing separator when it throws: line 322
`throw new MissingOptionException(buff.toString());` becomes line 323
`throw new MissingOptionException(buff.substring(0, buff.length() - 2));`.

## Mapping

All 14 mismatches (10 visible, 4 invisible) are one decision on line 322/323, all reason
`APTED("qualified_name")` (the APTED call inside the method matched by qualified name,
`solve_syntax_aware_matching.rs`).

- The ground truth matches the before call `buff.toString()` to the outer after call
  `buff.substring(...)` (`toString` -> `substring` update, `0, buff.length() - 2` inserted).
  omnidiff matches `buff.toString()` to the inner `buff.length()` instead (`toString` ->
  `length`, `()` -> `()`), and inserts the outer `buff.substring(` ... `)` around it. Both
  candidates are `method_invocation`s on `buff`; APTED prefers the one whose empty argument list
  matches `()`, but the outer call is the one that takes the original's place as the constructor
  argument. Wrong twin chosen. `matcher` (10 visible, 4 invisible: the two `method_invocation`
  and two `argument_list` parents).

## Painting

- Matcher, both presets: with omnidiff's mapping the before `buff.` is painted Move (5 bytes,
  None in the painting), the after `buff.substring(` and its closing `)` are painted Insert
  (15 + 1 bytes) and the inserted ` buff.length() ` is left unpainted (15 bytes, Insert in the
  painting). About 36 bytes per preset. `matcher`.
- Renderer, Minimal, 10 bytes (human mapping only; with omnidiff's mapping these bytes fall in
  the runs above): the painting paints the whole `toString` -> `substring` identifier as Update;
  the renderer narrows the update and leaves the shared `tring` unpainted on both sides.
  `renderer`.
- Renderer, Full, 4 bytes: the trailing `);` on line 322/323 is painted Move (its column shifted)
  where the Full painting leaves it unpainted; this survives with the human mapping. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: the APTED("qualified_name") resolution in
  `src/diff/solve_syntax_aware_matching.rs`: preferring the call that keeps the original's
  position (parent argument) over a nested call with the same receiver.
- Ground truth questionable: no.
