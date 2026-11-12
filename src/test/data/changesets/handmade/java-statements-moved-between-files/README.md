# A block of statements moved between functions in different files

- **Dataset:** handmade
- **Signals:** block-move
- **Repository, commit:** none. Written for OmniDiff.

The four statements that trim and shorten a subject leave `Sender.send` and arrive, unchanged, in `Composer.compose`, before its `return`. No declaration moves: the block crosses files below the level of a method.

What `git diff --name-status -M` reports for the two trees, as `source.json` records it:

- `M` `src/main/java/mail/Composer.java`
- `M` `src/main/java/mail/Sender.java`

## License

The files under `before/` and `after/` were written for this fixture and are not taken from any
other project. They are part of OmniDiff's own codebase and covered by its license, AGPL-3.0-only
(see `LICENSE` at the repository root).
