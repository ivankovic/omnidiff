# Accuracy analysis: rust-tiffany352-rink-rs-real-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 35 (28 visible) of 7270 nodes, 7275 graded, N:M floor 4 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`cli/src/config.rs` of rink-rs: the currency loading moves out (`try_load_currency` and its
helpers are gone, `load_cached_currency_if_current` is imported from `crate::currency`),
`config_path(name)` becomes `config_toml_path()`, and the `use` block is rewritten into one path
per line. `description.md`: "Requires N:M mapping."

## Mapping

- **Nested `use std::{collections::HashMap, fs::{read_to_string, File}};` (before lines 21-24),
  18 visible + 3 invisible.** The ground truth splits it 1:2 into `use std::collections::HashMap;`
  (after line 16) and `use std::fs::read_to_string;` (after line 18): `use`, `std`, `::` and `;`
  in an all-to-all group (12 visible), and the inner `collections`/`HashMap` and
  `fs`/`read_to_string` paths to the matching after lines (6 visible). omnidiff deletes the
  whole declaration and inserts both new ones (`APTED("fast_fallback")`). 4 visible are the N:M
  floor (one after copy of each shared token cannot be matched 1:1); the other 14 are the
  matcher's: a 1:1 mapping could still have kept one of the two after lines and the inner paths.
  `nm-floor` + `matcher`.
- **`use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};` (before line 17) -> `use
  std::io::ErrorKind;` (after line 19), 4 visible + 1 invisible.** Ground truth keeps the
  declaration and its `use`, `::`, `ErrorKind`, `;`. omnidiff deletes it and inserts the new line
  (`APTED("fast_fallback")`): with the list collapsed to a single path the two declarations have
  different shapes, and the last-resort fallback over the reshuffled import block did not pair
  them. `matcher`.
- **`"config.toml"` literal, 6 visible + 2 invisible.** Before line 308
  `config_path("config.toml")?`
  becomes `config_toml_path()?`, and the new `config_toml_path` (after line 27) has
  `path.push("config.toml");`. Ground truth deletes the old literal and inserts the new one;
  omnidiff pairs them as a move (`MovedSubtree`, byte-identical subtree). The literal's meaning did
  move from the call site into the new function, so both readings are defensible. `ambiguous`.
- **`try_load_currency(&config.currency, &mut ctx, &search_path)` (before line 364), 1
  invisible.** Ground truth maps the call to `load_cached_currency_if_current(...)` (after line
  335), now in a `let` instead of a `match` scrutinee; omnidiff deletes the call node
  (`APTED("qualified_name")`). Invisible only. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: `nm-floor`, `ambiguous`
- Where a fix would go: `apted::for_roots_fallback` (`fast_fallback`) pairing of restructured
  `use_declaration`s by shared path prefix; the N:M split itself is floor
- Ground truth questionable: no
