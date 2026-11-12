# A moved non-code file beside a code change

- **Dataset:** handmade
- **Signals:** non-code-move
- **Repository, commit:** none. Written for OmniDiff.

`site/img/logo.png` moves to `site/static/logo.png` with the same bytes, and `site/banner.py` changes the path it reads. The picture is an 8 by 8 pixel RGB gradient, generated for this fixture.

What `git diff --name-status -M` reports for the two trees, as `source.json` records it:

- `M` `site/banner.py`
- `R100` `site/img/logo.png` -> `site/static/logo.png`

## License

The files under `before/` and `after/` were written for this fixture and are not taken from any
other project. They are part of OmniDiff's own codebase and covered by its license, AGPL-3.0-only
(see `LICENSE` at the repository root).
