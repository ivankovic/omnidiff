# Accuracy analysis: c-openssl-openssl-a-few-changes

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 48 (30 visible) of 7174 nodes, 7174 graded, N:M floor 0 visible |
| Painting, Minimal | 228 bytes wrong of 25418: renderer 2, matcher 226 |
| Painting, Full | 468 bytes wrong of 25418: renderer 245, matcher 261 |
| Ground-truth invariant violations | none |

## The change

OpenSSL commit 5003be69 in `ssl/ech/ech_ssl_apis.c` hardens three ECH API functions against
lengths above `INT_MAX`: `SSL_ech_get1_retry_config` gets a braced `if` with a new condition and an
`ERR_raise`, a `BIO_write` length gets an `(int)` cast, one error code changes to
`ERR_R_PASSED_INVALID_ARGUMENT`, and in `SSL_set1_ech_config_list` the `ecl_len == 0` check gains
`|| ecl_len > INT_MAX`, its error code changes, and `BIO_new_mem_buf` gets an `(int)` cast.

## Mapping

All 48 mismatches are on the before side, in one function, `SSL_set1_ech_config_list` (before
lines 399-433, after lines 402-436). Category `matcher`.

- **Function shell and the two edited `if` statements deleted (APTED("qualified_name"), 24
  visible, 16 invisible).** The ground truth maps the `function_definition`, its body, the `int`
  return type (line 399), and every token of `if (ecl_len == 0) {...}` (lines 416-419) and of
  `if ((es_in = BIO_new_mem_buf(ecl, ecl_len)) == NULL || ...)` (lines 420-423) to their after
  counterparts: `if`, `(`, `)`, `{`, `}`, `||`, `==`, `es_in`, `BIO_new_mem_buf`, `ecl`,
  `ecl_len` are identical. omnidiff Deletes them all, with reason APTED("qualified_name"), the
  APTED call that `solve_qualified_name_groups` makes for the function pair matched by name. The
  untouched statements of the same function (`int rv = 0;`, `s = SSL_CONNECTION_FROM_SSL(ssl);`,
  the first two `if`s, the `err:` tail) are not in the list, so they are matched; only the root,
  its body, `int` and the two changed `if`s fall out. Even the unchanged `int` is Delete, which
  means the root pair itself was not kept. Why that APTED call gives up its own root pair is
  undetermined from the evidence; the two other edited functions,
  `SSL_ech_get1_retry_config` and `SSL_CTX_ech_set1_outer_alpn_protos`, map without a mismatch.
- **`ERR_raise(ERR_LIB_SSL, ERR_R_INTERNAL_ERROR)` on line 417 deleted (APTED("fast_fallback"),
  6 visible, 2 invisible).** The ground truth maps the call, `ERR_raise`, `ERR_LIB_SSL`, the
  punctuation and updates `ERR_R_INTERNAL_ERROR` to `ERR_R_PASSED_INVALID_ARGUMENT`. The
  qualified-name call left these unmatched and the final whole-root fallback then deleted them
  too, because its enclosing `if` had already been deleted.

## Painting

### Matcher-caused (Minimal 226 bytes, Full 261 bytes)

All are the deleted function shell above, rendered as Delete on before lines 399, 416-423 and
Insert on after lines 402, 419-426 (`int`, `if (`, `) {`, the `ERR_raise` lines, the
`BIO_new_mem_buf` condition, the three `||` and the closing `)`), where both paintings leave the
unchanged tokens unpainted (`Some(Delete/Insert) -> None`). In Full, `NULL` on lines 420/423 is
additionally painted Move (4+4 bytes). Category `matcher`.

### Renderer-caused

- **Minimal, 2 bytes, `None -> Insert`.** The new `{` and `}` of the braced `if` in
  `SSL_ech_get1_retry_config` (after lines 300 and 303) are painted Insert by the painting;
  omnidiff's Minimal punctuation filter drops them, with the human mapping too. Category
  `renderer`.
- **Full, 139 bytes, `None -> Move/Insert`.** In `SSL_ech_get1_retry_config` the Full painting
  marks `    if (s == NULL || ec == NULL || eclen == NULL` (line 299 both sides; the condition
  is now the left operand of a larger `||`) and `        goto err;` (before line 300, after line
  302; now inside a new block) as Move, and the new indentation and space on after line 300 as
  Insert. omnidiff leaves the reparented text unpainted. Category `renderer`.
- **Full, 106 bytes, `Move -> None`.** Line tails pushed right or reparented by an edit earlier on
  the line are painted Move by omnidiff and left unpainted by the painting:
  `s->ext.ech.returned_len) <= 0` after the inserted `(int)` (before 319, after 322), `);` after
  the renamed error code (before 358, after 361), and `) {`, `);`, `ecl_len)) == NULL` on before
  lines 416, 417, 420 / after lines 419, 420, 423 (the latter overlap with the matcher runs under
  omnidiff's own mapping). This is painting rule 3 (an identical line tail stays unpainted)
  against Full's displacement painting. Category `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: `solve_syntax_aware_matching::solve_qualified_name_groups` and the
  `apted::for_nodes` call it makes (its pre-matches `prematch_identical_statement_siblings` and the
  `ContainmentCtx` sibling-order check in `apted/common/resolve.rs` are where to look; not
  verified): the
  function root and the two edited `if`s should survive; for Full, `diff/text.rs` handling of
  reparented-in-place text and displaced line tails.
- Ground truth questionable: no
