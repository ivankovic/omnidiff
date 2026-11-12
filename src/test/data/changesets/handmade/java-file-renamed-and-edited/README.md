# A file renamed and edited

- **Dataset:** handmade
- **Signals:** rename-edited
- **Repository, commit:** none. Written for OmniDiff.

The class `PriceCalc` becomes `PriceCalculator`: the file is renamed, the class and constructor names change with it, and `total` now rounds. `Checkout.java` follows the new name. The edit is large enough for the small file that git reports a delete and an add, not a rename.

What `git diff --name-status -M` reports for the two trees, as `source.json` records it:

- `M` `src/main/java/shop/Checkout.java`
- `D` `src/main/java/shop/PriceCalc.java`
- `A` `src/main/java/shop/PriceCalculator.java`

## License

The files under `before/` and `after/` were written for this fixture and are not taken from any
other project. They are part of OmniDiff's own codebase and covered by its license, AGPL-3.0-only
(see `LICENSE` at the repository root).
