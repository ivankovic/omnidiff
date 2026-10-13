# Accuracy analysis: rust-yannjor-krabby-actual-normal-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 442 (323 visible) of 2922 nodes, 3013 graded, N:M floor 66 visible |
| Painting, Minimal | 1611 bytes wrong of 10012: renderer 489, matcher 1469 |
| Painting, Full | 1857 bytes wrong of 10012: renderer 347, matcher 1670 |
| Ground-truth invariant violations | 9 |

## The change

krabby `src/main.rs`: the four options shared by `Name` and `Random` (`info`, `shiny`,
`no_title`, `padding_left`) are extracted into a new `struct CommonArgs` that both embed as
`#[clap(flatten)] common`; a `List` args struct and `Random.no_variant` are added; the
`list_pokemon_names` / `show_pokemon_by_name` / `show_random_pokemon` functions leave the file
(they become `PokemonDatabase` methods elsewhere) and `main` calls them as methods.
`description.md`: "Requires N:M mapping".

## Mapping

- **`CommonArgs` fields, 198 visible + 75 invisible (all-to-all groups).** The ground truth maps
  each field of `CommonArgs` (after lines 39-53) 2:1 from the same field in `Name` (before lines
  49-63) and in `Random` (before lines 72-82, 96-98). omnidiff inserts the whole of `CommonArgs`
  (`APTED("fast_fallback")`) because the before fields were already used up inside `Name` and
  `Random` (next item). The N:M floor is 66 visible (one source per field); the other 132 are
  `matcher`.
- **Fields inside `Name` and `Random` relabelled positionally, 76 visible + 32 invisible.** In
  `Random`, omnidiff maps `info`/`shiny`/`no_title` (before 72-82) onto `no_mega`/`no_gmax`/
  `no_regional` (after 83-93) as Updates, `no_mega`/`no_gmax` (before 84-90) onto `no_variant`/
  `common` (after 95-100), and deletes `no_regional` (before 94), instead of keeping the three
  byte-identical `no_*` fields. In `Name`, `shiny` (before 49-51) becomes `#[clap(flatten)]
  common` (after 73-74). All `APTED("qualified_name")`: with unit-cost updates, relabelling a
  field of the same shape (two leaf updates) is cheaper than deleting the extracted fields and
  inserting the new ones, so the ordered edit distance prefers the shifted alignment. `matcher`.
- **`#[derive(Debug, Args)]`, 36 visible + 12 invisible.** The before attributes of `Name` (line
  39) and `Random` (line 66) go to the new `CommonArgs` (after 37) and `List` (after 56) in
  omnidiff (`APTED("fast_fallback")`, identical text, positional), not to `Name` (after 63) and
  `Random` (after 77). Wrong twin among identical attributes. `matcher`.
- **`main`, 10 visible.** `list_pokemon_names(pokemon)` -> `pokemon_db.list_pokemon_names(...)`
  (before 227-229 -> after 113-115): ground truth maps the function name onto the method name;
  omnidiff maps it onto the new receiver `pokemon_db` (Update), and `load_pokemon` onto
  `PokemonDatabase` rather than `load` (after line 110). `APTED("qualified_name")`. `matcher`.
- **Deleted functions, 3 visible.** `list_pokemon_names` (before 105) paired with `List` (after
  57) and `contains` (before 195) with `Args` (after 77), by `APTED("fast_fallback")` as
  `MatchButNotIdentical`: the fallback pairs leaves of different kinds (`identifier` with
  `type_identifier`, `field_identifier` with `identifier`). `matcher`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Painting

Two paintings (`Minimal`, `Full`). By region (bytes where omnidiff's rendering is wrong):

| | Minimal | Full |
|---|---|---|
| structs, matcher-only | 989 | 1352 |
| structs, also with human mapping | 279 | 82 |
| `main`, matcher-only | 179 | 175 |
| `main`, also with human mapping | 134 | 221 |
| deleted functions, matcher-only | 30 | 27 |

- **Structs, matcher (989 / 1352 bytes).** The mapping above, rendered: `Name`'s fields Delete
  where the painting has Move (into `CommonArgs`), `CommonArgs` Insert where it has Move, the
  relabelled `Random` fields Update where the painting leaves them alone, the new
  `no_variant`/`common` lines unpainted or Update where the painting inserts them, and the
  `derive` lines. `matcher`.
- **Structs, shared (279 / 82 bytes).** The painting paints `Random`'s `info`, `shiny`,
  `no_title` and `padding_left` (before 72-82, 96-98) Move, as the second source of each 2:1
  field; rendering the human mapping leaves them unpainted or Delete. One-to-one rendering of
  the N:M ground truth. `nm-floor`.
- **`main`, matcher (179 / 175 bytes).** The receiver/method mix-up above: `pokemon_db` painted
  Update where the painting inserts it, `list_pokemon_names` etc. Update/Insert where the
  painting leaves the method names alone. `matcher`.
- **`main`, shared (134 / 221 bytes).** `let pokemon_db` -> `let pokemon_db_file` and
  `pokemon` -> `pokemon_db` (before 223-224, after 109-110): omnidiff paints the renamed
  identifiers as narrowed Updates and the unchanged rest of each line (`= Asset::get(...)`, `=`,
  `=>`) Move; the Minimal painting inserts only `_file` / `_db` and leaves the rest. The
  ground-truth lint flags this region: [16] Minimal paints `load_pokemon` against `load`, and
  the Full painting does not paint renamed identifiers whole (8 hits). Mostly `renderer`, with
  the [16] spans `ground-truth`.

## Verdict

- Primary cause: `matcher`
- Other causes: `nm-floor`, `renderer`, `ground-truth`
- Where a fix would go: the `qualified_name` APTED call site (positional relabelling of struct
  fields when some fields are extracted; an identical-subtree anchor before APTED), and
  `apted::for_roots_fallback` twin choice for identical attributes
- Ground truth questionable: yes, narrowly: 9 lint hits ([16]) on the `main` renames (Minimal
  paints `load` inside `load_pokemon`; Full does not paint renamed identifiers whole)
