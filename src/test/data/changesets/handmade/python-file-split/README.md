# A file split into two

- **Dataset:** handmade
- **Stratum:** split or merge
- **Repository, commit:** none. Written for OmniDiff.

The two rendering functions of `inventory.py` move to a new `rendering.py`. What stays is untouched, and so are the functions that move.

What `git diff --name-status -M` reports for the two trees, as `source.json` records it:

- `M` `inventory.py`
- `A` `rendering.py`

Not solved yet: there is no `human_changeset.json`.

## License

The files under `before/` and `after/` were written for this fixture and are not taken from any
other project. They are part of OmniDiff's own codebase and covered by its license, AGPL-3.0-only
(see `LICENSE` at the repository root).
