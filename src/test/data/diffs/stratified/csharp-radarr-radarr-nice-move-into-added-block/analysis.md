# Accuracy analysis: csharp-radarr-radarr-nice-move-into-added-block

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 7 (5 visible) of 1174 nodes, 1174 graded, N:M floor 0 visible |
| Painting, Minimal | 466 bytes wrong of 5665: renderer 462, matcher 18 |
| Painting, Full | 721 bytes wrong of 5665: renderer 715, matcher 18 |
| Ground-truth invariant violations | 4 (three rule [16] on `RadarrAuth`/`Auth` of line 39/42, one rule [5] on line 37) |

## The change

The cookie options used to be configured in the lambda passed to `.AddCookie(…, options => {…})` at
the end of the `return services.AddAuthentication()…` chain (before lines 33-45). The commit moves
the lambda body into a new statement
`services.AddOptions<CookieAuthenticationOptions>(…).Configure<IConfigFileProvider>((options,
configFileProvider) => {…});` placed above the chain (after lines 35-47), makes the cookie name
`$"{instanceName}Auth"` from the configured instance name, and adds three usings. The ground truth
deletes the old lambda and block, inserts the new ones, and maps the six `options.*` statements
across as moved.

## Mapping

- Before line 39 / after line 42, 5 visible + 2 invisible: the ground truth maps
  `options.Cookie.Name = "RadarrAuth";` to `options.Cookie.Name = $"{instanceName}Auth";` (an
  update; the string literal becomes an interpolated string). omnidiff keeps `options.Cookie.Name`
  but deletes the `=`, both `"`, the string content `RadarrAuth`, the `;`, and the enclosing
  `assignment_expression` and `expression_statement`, all with reason `APTED("qualified_name")`
  (the whole-method APTED call in `solve_qualified_name_groups`,
  src/diff/solve_syntax_aware_matching.rs). The five byte-identical sibling statements are
  matched; only the statement whose literal changed node type is left to APTED, which pairs the
  identical left-hand side and gives up on the rest. Why the edit-distance solution prefers
  delete + insert over an update of the statement was not traced further. `matcher`.

## Painting

Matcher-owned, 13-18 bytes in each preset (Delete/Insert -> Move): `Auth";` on before line 39 and
`"`, `Auth";` on after line 42, which the painting matches and omnidiff paints Delete/Insert because
of the broken statement above. `matcher`.

Renderer-owned (identical with the human mapping), Minimal 462 bytes, Full 715 bytes:

- None -> Move, the six `options.*` statements on before lines 39-44 / after lines 42-47 (Minimal
  about 458 bytes, Full about 690 with the 20-column indentation the Full painting matches too).
  This is the point of the fixture: the statements moved out of the deleted lambda into the
  inserted one. omnidiff's `identical_or_move` (src/diff/text.rs) paints an `Identical`-mapped
  node Move only for a meaningful column change or a multi-row node that crossed backwards. Each
  statement is single-row, keeps column 20, and only shifts down three rows, which the renderer
  reads as "an unrelated edit above", so all six render unchanged although their parent block was
  deleted and inserted. `renderer`.
- None -> Delete/Insert, Minimal 3 bytes: the lone `{` of before line 38 and after line 37 and the
  `}` of before line 45, dropped by the Minimal punctuation filter. `renderer`.
- Delete -> None, 1 byte per preset, before line 37: the space between `,` and `options =>`.
  omnidiff paints `, options =>` as one Delete; the painting paints `,` and `options =>` as two
  entries and leaves the space between them unpainted, which invariant [5] flags. `ground-truth`.
- Full only: the 16-byte indentation of before line 45 (`})`), which the Full painting deletes and
  omnidiff leaves unpainted, and the closing `)` of `.AddCookie(...)` (before line 45, after
  line 53), which omnidiff paints Move (it changed row and column) while the painting leaves both
  brackets of that call unpainted. 18 bytes. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`, `ground-truth`
- Where a fix would go: `identical_or_move` in src/diff/text.rs, which never paints a single-row,
  same-column node Move even when it was reparented into an inserted block; secondarily the
  method-body APTED call (`solve_qualified_name_groups`) that breaks the cookie-name statement
- Ground truth questionable: yes, minor: the paintings treat `Auth` of `RadarrAuth` as its own
  matched span (three invariant [16] violations) and leave one space unpainted between two Delete
  entries on line 37 (invariant [5]); neither carries many bytes
