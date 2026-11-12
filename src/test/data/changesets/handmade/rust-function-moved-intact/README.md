# A function moved intact to another existing file

- **Dataset:** handmade
- **Stratum:** block move
- **Repository, commit:** none. Written for OmniDiff.

`perimeter` leaves `src/geometry.rs` for the end of `src/measure.rs`, byte for byte. Nothing else changes in either file.

What `git diff --name-status -M` reports for the two trees, as `source.json` records it:

- `M` `src/geometry.rs`
- `M` `src/measure.rs`

Solved by hand as a worked example of the format: both files are `same`, and the function is one cross-file `identical_with_children` entry, painted as a `Match` from one file to the other.

## License

The files under `before/` and `after/` were written for this fixture and are not taken from any
other project. They are part of OmniDiff's own codebase and covered by its license, AGPL-3.0-only
(see `LICENSE` at the repository root).
