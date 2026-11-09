# Accuracy analysis: c-openssl-openssl-big-multi-map-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 114 (62 visible) of 19067 nodes, 19087 graded, N:M floor 10 visible |
| Painting, Minimal | 886 bytes wrong of 74869: renderer 629, matcher 260 |
| Painting, Full | 499 bytes wrong of 74869: renderer 278, matcher 435 |
| Ground-truth invariant violations | 12 (2 of kind [9], 10 of kind [16]) |

## The change

OpenSSL commit 2ea6e785 in `crypto/pkcs7/pk7_doit.c` drops the legacy fallback after
`EVP_MD_fetch` / `EVP_CIPHER_fetch` at five sites. Each site had a temporary (`fetched`,
`evp_md`, `evp_cipher`, `fetched_md`) and a `const` pointer (`md`, `cipher`); the two
declarations merge into one non-const `EVP_MD *md = NULL;` (or `EVP_CIPHER *cipher = NULL;`), the
fetch is assigned to `md`/`cipher` directly, the `if (x != NULL) md = x; else md = ...;` block and
the `ERR_set_mark` / `ERR_clear_last_mark` / `ERR_pop_to_mark` calls are deleted, and the `_free`
calls take the surviving name.

## Mapping

All 114 mismatches come from the APTED("qualified_name") call (`solve_qualified_name_groups`)
for the three functions, and all follow one pattern: the ground truth keeps the variable that
keeps its name, omnidiff keeps the declaration and the statement that keep their shape.

- **Declarations (22 visible, 12 invisible; before lines 99-100, 443-444, 1068-1069).** The ground
  truth deletes `EVP_MD *fetched` / `EVP_MD *evp_md` / `EVP_MD *fetched_md` and maps
  `const EVP_MD *md;` to after `EVP_MD *md = NULL;` (dropping `const`; the `= NULL` comes from the
  deleted declaration). omnidiff maps the temporary's declaration `EVP_MD *fetched = NULL;` to
  `EVP_MD *md = NULL;` with `fetched -> md` as an Update and deletes `const EVP_MD *md;`. That is
  the cheaper tree edit (one rename against a deleted qualifier plus a moved initializer). Where
  both twins carry `= NULL` (`EVP_CIPHER *evp_cipher = NULL;` / `const EVP_CIPHER *cipher = NULL;`,
  before lines 445-446), omnidiff picks the same twin as the ground truth, so its choice follows
  edit cost, not name identity. Category `ambiguous`.
- **Fetch assignments (10 visible, 10 invisible; before lines 110, 501, 522, 558, 1143).** The
  ground truth deletes the `fetched =` / `evp_cipher =` / `evp_md =` / `fetched_md =` left-hand
  side and maps the after `md =` / `cipher =` to the `md = fetched;` and `md = EVP_get_...;`
  statements of the deleted `if/else` (a 2:1 all-to-all group), while the `EVP_MD_fetch(...)` call
  keeps its own mapping. omnidiff maps the whole `fetched = EVP_MD_fetch(...)` statement to
  `md = EVP_MD_fetch(...)` with the identifier as an Update, which is also what a line diff shows.
  Category `ambiguous`.
- **The 2:1 groups (30 visible, 30 invisible).** For each of the five sites, `md`, `=`, the
  assignment and the expression statement of both `md = fetched;` and `md = EVP_get_...;` are
  expected to match within the group; omnidiff deletes all of them (they lost to the fetch
  assignment above). 10 of these visible mismatches (one `md` and one `=` per site) are the N:M
  floor (`nm-floor`); the other 20 follow from the `ambiguous` choice above.

The ground truth's reading is applied consistently at all five sites; omnidiff's reading is
equally consistent and is the minimal edit. Neither is wrong on its face.

## Painting

### Matcher-caused (Minimal 260 bytes, Full about 440 bytes)

The rendering of the mapping choices above: the temporary's declaration painted None or Move where
the painting deletes `EVP_MD *fetched` and moves `EVP_MD *md`; `const EVP_MD *md;` painted Delete
where the painting moves `EVP_MD *md` and `;`; `fetched =` painted Update where the painting
deletes it; `md = ` of the deleted `if/else` painted Delete where the painting moves it; and the
after `md =` / `cipher =` painted Update or None where the painting marks Move. Labels
`None/Update/Delete -> Delete/Move`. Category `ambiguous` (it follows the mapping choice).

### Present with the human mapping too, Minimal (629 bytes)

- **581 bytes, `None -> Move`: N:M-transferred parts not painted.** With the human mapping, the
  Minimal rendering leaves unpainted the parts the ground truth moves between statements: the
  `= NULL;` taken from the deleted declaration (before lines 99, 443, 1069 / after 99, 435, 1037,
  42 bytes), the `EVP_MD_fetch(...)` / `EVP_CIPHER_fetch(...)` right-hand sides that move from the
  deleted `fetched = ...` statement into `md = ...` (484 bytes), the `;` of `const EVP_MD *md;`
  (3 bytes), and `EVP_CIPHER *cipher = NULL;` (before line 446, after 436, 52 bytes). Full renders
  the fetch-call and `cipher` moves (they do not appear in the Full human runs), so for those this
  is Minimal's own suppression of the Move; the `= NULL` at line 99 is left unpainted in Full too.
  Category `renderer`, except the last 52 bytes (see ground truth below).
- **4 bytes, `None -> Update`.** `EVP_MD_free(fetched)` -> `EVP_MD_free(md)` (before lines 126,
  129 / after 118, 121): the painting updates the whole word, omnidiff narrows the Update and leaves
  the shared trailing `d` unpainted, a common suffix inside a word. Category `renderer`.
- **36 bytes, `None -> Update`.** `evp_md` / `evp_cipher` / `fetched_md` -> `md` / `cipher` in
  `_free` calls and uses (before 572, 576, 706, 710, 1184 / after 541, 545, 675, 679, 1144): the
  painting updates the whole identifier, omnidiff paints only the dropped `evp_` / `fetched_`
  word. invariants.txt [16] flags these painting entries as wrong for a Minimal reading. Category
  `ground-truth`.
- **8 bytes, `Delete -> None`.** ` = NULL;` of the deleted `EVP_CIPHER *evp_cipher = NULL;`
  (before line 445): omnidiff paints it Delete; the Minimal painting deletes only
  `EVP_CIPHER *evp_cipher` and leaves the rest of the deleted declaration unpainted (the Full
  painting deletes it). Category `ground-truth`.

### Present with the human mapping too, Full (278 bytes)

- **250 bytes, `None -> Delete/Move`.** Indentation and single separating spaces of the merged
  declarations and of the deleted `md = ...;` statements (for example the leading whitespace of
  before lines 113, 115, 503, 505, 560, 562, 1146, 1148 painted Delete, and of after lines 99, 108,
  435, 490, 504, 533, 1037, 1110 painted Move), plus ` = NULL` on line 99 of both sides (7+7
  bytes) and a few single `;` runs. omnidiff's Full rendering of the human N:M mapping does not
  paint the whitespace around partially moved statements. Category `renderer`.
- **28 bytes, `Move -> None`.** `);` after a renamed argument in the `_free` calls (before lines
  126, 129, 572, 576, 706, 710, 1184 and their after lines): omnidiff's Full rendering paints the
  displaced tail Move, the painting leaves it (painting rule 3). Category `renderer`.

## Verdict

- Primary cause: `ambiguous`
- Other causes: `renderer`, `nm-floor`, `ground-truth`
- Where a fix would go: none for the mapping if both readings stand; if the name-identity
  reading is the intended one, a declarator-name tie-break in the APTED("qualified_name") call of
  `solve_syntax_aware_matching::solve_qualified_name_groups`. For Minimal, the rendering of Moves
  that an N:M mapping transfers between statements (`diff/text.rs`).
- Ground truth questionable: yes. (1) invariants.txt [9]: at before line 444 the tree mapping
  deletes the `;` of `const EVP_MD *md;` while both paintings move it; at the first site (line
  100) the tree mapping maps that `;`, so the second site disagrees with the first and with its own
  painting. (2) invariants.txt [16]: the Minimal painting updates whole identifiers
  (`evp_md` -> `md`, 10 entries) where only the prefix word differs. (3) The Minimal painting
  leaves ` = NULL;` of the deleted declaration on before line 445 unpainted. (4) The Minimal
  painting marks `EVP_CIPHER *cipher = NULL;` (before 446, after 436) as Move although only a
  leading `const` was removed, which looks inconsistent with painting rule 3 (an identical line
  tail stays unpainted).
