# A directory renamed, one file edited

- **Dataset:** handmade
- **Stratum:** directory move
- **Repository, commit:** none. Written for OmniDiff.

`lib/` becomes `math/`. `clamp.ts` and `lerp.ts` keep their bytes; `index.ts` gains an exported constant.

What `git diff --name-status -M` reports for the two trees, as `source.json` records it:

- `R100` `lib/clamp.ts` -> `math/clamp.ts`
- `R068` `lib/index.ts` -> `math/index.ts`
- `R100` `lib/lerp.ts` -> `math/lerp.ts`

Not solved yet: there is no `human_changeset.json`.

## License

The files under `before/` and `after/` were written for this fixture and are not taken from any
other project. They are part of OmniDiff's own codebase and covered by its license, AGPL-3.0-only
(see `LICENSE` at the repository root).
