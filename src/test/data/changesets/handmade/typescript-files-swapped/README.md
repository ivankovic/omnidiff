# Two files swapping contents

- **Dataset:** handmade
- **Signals:** rename-unedited
- **Repository, commit:** none. Written for OmniDiff.

`themes/default.ts` and `themes/alternate.ts` exchange their contents, byte for byte. Each path is modified; nothing is new.

What `git diff --name-status -M` reports for the two trees, as `source.json` records it:

- `M` `themes/alternate.ts`
- `M` `themes/default.ts`

## License

The files under `before/` and `after/` were written for this fixture and are not taken from any
other project. They are part of OmniDiff's own codebase and covered by its license, AGPL-3.0-only
(see `LICENSE` at the repository root).
