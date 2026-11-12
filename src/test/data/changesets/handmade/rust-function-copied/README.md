# A function copied; the original stays

- **Dataset:** handmade
- **Signals:** copy
- **Repository, commit:** none. Written for OmniDiff.

`round_to` is copied from `src/units.rs` to the end of `src/report.rs`. `src/units.rs` is **untouched** (`source.json` marks it): it is in the changeset because it is where the copy comes from, and no VCS line mentions it.

What `git diff --name-status -M` reports for the two trees, as `source.json` records it:

- `M` `src/report.rs`

Solved by hand as a worked example of the format: `round_to` in `src/units.rs` has two partners, itself in place and the copy in `src/report.rs` (a one-to-many match), and the copy is painted as a `Match` from the original.

## License

The files under `before/` and `after/` were written for this fixture and are not taken from any
other project. They are part of OmniDiff's own codebase and covered by its license, AGPL-3.0-only
(see `LICENSE` at the repository root).
