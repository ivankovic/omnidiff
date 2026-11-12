# Two files merged into one

- **Dataset:** handmade
- **Signals:** block-move, rename-edited, split-or-merge
- **Repository, commit:** none. Written for OmniDiff.

`cache/get.go` and `cache/put.go`, one method each, become `cache/access.go` holding both. The second file's `package` line is the only code that goes away. Git pairs the new file with one of the two and calls the other deleted.

What `git diff --name-status -M` reports for the two trees, as `source.json` records it:

- `R059` `cache/get.go` -> `cache/access.go`
- `D` `cache/put.go`

## License

The files under `before/` and `after/` were written for this fixture and are not taken from any
other project. They are part of OmniDiff's own codebase and covered by its license, AGPL-3.0-only
(see `LICENSE` at the repository root).
