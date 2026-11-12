# Control: three files changed, nothing crossing files

- **Dataset:** handmade
- **Stratum:** control
- **Repository, commit:** none. Written for OmniDiff.

Three files of one package each get their own edit: a field added, a function given a second return value and an empty check, a doc comment added. No code moves between them, so the right answer is three single-file diffs.

What `git diff --name-status -M` reports for the two trees, as `source.json` records it:

- `M` `queue/len.go`
- `M` `queue/pop.go`
- `M` `queue/queue.go`

Not solved yet: there is no `human_changeset.json`.

## License

The files under `before/` and `after/` were written for this fixture and are not taken from any
other project. They are part of OmniDiff's own codebase and covered by its license, AGPL-3.0-only
(see `LICENSE` at the repository root).
