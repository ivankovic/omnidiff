# A file renamed and not edited

- **Dataset:** handmade
- **Signals:** rename-unedited
- **Repository, commit:** none. Written for OmniDiff.

`src/helpers.ts` becomes `src/format.ts` with the same bytes. `src/listing.ts` changes only the path it imports from.

What `git diff --name-status -M` reports for the two trees, as `source.json` records it:

- `R100` `src/helpers.ts` -> `src/format.ts`
- `M` `src/listing.ts`

## License

The files under `before/` and `after/` were written for this fixture and are not taken from any
other project. They are part of OmniDiff's own codebase and covered by its license, AGPL-3.0-only
(see `LICENSE` at the repository root).
