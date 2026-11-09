# Accuracy analysis: c-nginx-add-typedef

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 50 (32 visible) of 11141 nodes, 11141 graded, N:M floor 0 visible |
| Painting, Minimal | 178 bytes wrong of 42218: renderer 85, matcher 149 |
| Painting, Full | 271 bytes wrong of 42218: renderer 228, matcher 213 |
| Ground-truth invariant violations | 6 |

## The change

nginx `src/event/ngx_event_openssl_cache.c`: a `NGX_SSL_CACHE_DISABLED` define and a
`ngx_ssl_cache_pwd_t { pwd; encrypted:1; }` typedef are added; the cache's `create` callbacks
take `&data` instead of `data`; in the private-key loader (before lines 475-563) the password
cursor `ngx_str_t *pwd` is replaced by a `ngx_ssl_cache_pwd_t cb_data, *pwd` and the loop
updates `cb_data.pwd`; and the password callback (before line 574) reads its
`ngx_ssl_cache_pwd_t *data = userdata` and sets `data->encrypted`.

## Mapping

- **Two swapped statements, 13 visible + 6 invisible** (before lines 535-536 -> after 546-547).
  `tries = passwords->nelts; pwd = passwords->elts;` become `cb_data.pwd = (*passwords)->elts;
  tries = (*passwords)->nelts;`. Ground truth follows the content (`tries` to `tries`, `pwd` to
  the field `cb_data.pwd`); omnidiff pairs them by position (`APTED("qualified_name")`): an
  ordered mapping cannot keep both crossing pairs, so `tries` becomes `cb_data` and `pwd` becomes
  `tries`, with `nelts`/`elts` swapped. `matcher`.
- **`pwd++` -> `cb_data.pwd++`, 2 visible** (before 555 -> after 568). Ground truth maps `pwd` to
  the field `pwd`; omnidiff maps it to `cb_data` (identifier to identifier; the field is a
  `field_identifier`). `matcher`.
- **`return NULL;` -> `return value;`, 1 visible** (before 233 -> after 242): ground truth updates
  `NULL` to `value`; omnidiff deletes and inserts (different node kinds). `matcher`.
- **The two `pwd` declarations, 16 visible + 9 invisible.** Before line 479 `ngx_str_t *pwd;`
  (loader) and before line 574 `ngx_str_t *pwd = userdata;` (callback). Ground truth: the first
  is deleted; the second becomes after line 593 `ngx_str_t *pwd;` and after line 591
  `ngx_ssl_cache_pwd_t *data = userdata;` is new. omnidiff: the first moves to after 593
  (`MovedSubtree`, byte-identical), the second becomes after 591 (`APTED("qualified_name")`, type
  and name updated, `= userdata` kept). Both readings are defensible, and the ground truth's own
  painting sides partly with omnidiff: it paints `= userdata` Move from 574 to 591 and
  `*pwd;` Move from 479 to after 490, where its tree reads delete/insert (`invariants.txt` [9]).
  `ambiguous`, with `ground-truth` inconsistency.

## Painting

- **Matcher, ~107 bytes Minimal / ~122 Full:** the groups above rendered (`tries`/`pwd`/`cb_data`
  Updates where the painting leaves or inserts them, `ngx_ssl_cache_pwd_t  *data` unpainted
  where the painting inserts it, `ngx_str_t  *pwd;` on after 593 painted Move, `NULL`/`value`
  Delete/Insert where the painting updates). `matcher` / `ambiguous`.
- **Painting against its own tree, ~42 bytes Minimal / ~40 Full (also wrong with the human
  mapping):** `= userdata` and `elts` painted Move, ` *pwd;` on after 490 painted Move
  ([9]), and `pwd = ` on after line 548 left unpainted although the tree says it is new ([11]).
  `ground-truth`.
- **Renderer, ~29 bytes Minimal / ~110 Full (also with the human mapping):** tokens pushed
  right by the inserted `(*`, `&` and `*` (`passwords`, `=`, `data);`, `) {`, `->nelts;`,
  `passwords = data;`) painted Move where the painting leaves them, mostly in Full, and the new
  `**` / ` = ` left unpainted. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `ambiguous`, `ground-truth`, `renderer`
- Where a fix would go: the `qualified_name` APTED call site (swapped sibling statements, which
  an ordered mapping cannot keep; identifier -> field_identifier relabel); Full displacement in
  `diff::text`
- Ground truth questionable: yes: 6 invariant violations; the painting moves `= userdata` and
  `*pwd;` where the tree deletes/inserts them ([9]) and leaves a new `pwd` unpainted ([11])
