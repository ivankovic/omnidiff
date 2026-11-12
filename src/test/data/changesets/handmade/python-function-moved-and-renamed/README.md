# A function moved and renamed

- **Dataset:** handmade
- **Stratum:** moved and edited
- **Repository, commit:** none. Written for OmniDiff.

`parse_body` leaves `app/handlers.py` and arrives in `app/codec.py` as `decode_body`, with the same body. Its caller follows the new name, and an import replaces the one the function needed.

What `git diff --name-status -M` reports for the two trees, as `source.json` records it:

- `M` `app/codec.py`
- `M` `app/handlers.py`

Not solved yet: there is no `human_changeset.json`.

## License

The files under `before/` and `after/` were written for this fixture and are not taken from any
other project. They are part of OmniDiff's own codebase and covered by its license, AGPL-3.0-only
(see `LICENSE` at the repository root).
