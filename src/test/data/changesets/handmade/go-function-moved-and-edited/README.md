# A function moved and edited inside

- **Dataset:** handmade
- **Signals:** moved-and-edited
- **Repository, commit:** none. Written for OmniDiff.

`clientAddress` leaves `server/routes.go` for `server/request.go`, and on the way its forwarded-header branch is rewritten. `request.go`'s import becomes a group to take `strings`.

What `git diff --name-status -M` reports for the two trees, as `source.json` records it:

- `M` `server/request.go`
- `M` `server/routes.go`

## License

The files under `before/` and `after/` were written for this fixture and are not taken from any
other project. They are part of OmniDiff's own codebase and covered by its license, AGPL-3.0-only
(see `LICENSE` at the repository root).
