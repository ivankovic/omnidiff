# Accuracy analysis: java-defects4j-cli-20-posixparser

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 53 (38 visible) of 2067 nodes, 2083 graded, N:M floor 12 visible |
| Painting, Minimal | 122 bytes wrong of 20598: renderer 7, matcher 115 |
| Painting, Full | 377 bytes wrong of 20598: renderer 54, matcher 372 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-20 in `PosixParser.flatten`, long-option branch (before lines 114-122, after lines
114-127). The three `token.indexOf('=')` calls collapse into `int pos = token.indexOf('=');` and
`String opt = pos == -1 ? token : token.substring(0, pos);`. The `if (token.indexOf('=') != -1)`
becomes `if (!options.hasOption(opt) && stopAtNonOption) { process(token); }`, and the `else` now
adds `opt` and, when `pos != -1`, the value in a new nested `if`. Marko's `description.md`:
"Requires N:M mapping".

## Mapping

All printed reasons, including those on the after-side "descendant" lines, are
`APTED("qualified_name")`.

- **Three `token.indexOf('=')` -> one (12 visible, 4 invisible).** The ground truth maps the three
  calls (before lines 114, 116, 117) to the one in `int pos = ...` (after line 114) as a 3:1
  group. omnidiff maps one and deletes the copies on lines 116 and 117. This is the fixture's N:M
  floor (12 visible). `nm-floor`.
- **The old `if`/`else` skeleton (7 visible, 3 invisible).** `if`, `(`, `)`, `{`, `}`, `else`
  (before lines 114-119) and `!=` (-> `==` in the new ternary, after line 115). The ground truth
  maps them to the new `if (!options.hasOption(opt) && stopAtNonOption)` and its `else`
  (after lines 117-121). omnidiff deletes them and inserts the new ones; the outer `if
  (token.startsWith("--"))` stays matched. `matcher`.
- **The `else` block (18 visible, 8 invisible).** The ground truth maps the old `else {
  tokens.add(token); }` (before lines 120-122) to the new `else` block, `tokens.add(token)` ->
  `tokens.add(opt)` (after line 123). omnidiff maps it to the new nested `if (pos != -1) {
  tokens.add(token.substring(pos + 1)); }` (after lines 124-126): its `{`, `}`, and
  `tokens.add(token` ... `);` go to the inserted block, because the nested call shares the
  longer prefix. A brand-new block takes the old one's place. `matcher`.
- **`token` of `token.length()` (1 visible).** Before line 117 col 72; the ground truth deletes it
  with the rest of the line, omnidiff pairs it as an Update. `matcher`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Painting

Byte counts are from the runs; the table's split is the reference.

- N:M, Minimal 38 / Full 36 bytes, Delete -> Move/None: the two extra `token.indexOf('=')` calls
  (before lines 116-117), which the paintings paint `Move` into `pos`. `nm-floor`.
- Matcher, Minimal 29 / Full about 171 bytes: the `if`/`else` skeleton painted Delete (before
  114-119) and Insert (after 117-122, 127), with indentation under Full.
- Matcher, Minimal 48 / Full 137 bytes: the `else` block: before `tokens.add(token` painted
  `Move` and after line 125's nested call painted `Move` where the painting inserts it; `{`, `}`,
  `);` of the nested `if` unpainted; `tokens.add(opt);` painted Update where the painting keeps it.
- Matcher, 6 bytes: ` token` of `token.length()` (before line 117) not painted Delete.
- Renderer (also with the human mapping), Minimal 7 / Full about 27 bytes in `real`: the painting
  paints ` -1` (and `)`) as part of the `!=` -> `==` Update (before line 114, after line 115),
  omnidiff narrows it away; the new `;` of `int pos = ...;` (after line 114) left unpainted; under
  Full the indentation of after line 114. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `nm-floor`, `renderer`
- Where a fix would go: `APTED("qualified_name")`: it gives the old `else` block to a newly
  inserted nested block and deletes the old `if` skeleton; N:M support (`solve_multi_maps`) for
  the 3:1 call collapse
- Ground truth questionable: no
