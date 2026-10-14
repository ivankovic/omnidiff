# Accuracy analysis: json-immich-app-immich-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1062 nodes, 1062 graded, N:M floor 0 visible |
| Painting, Minimal | 46 bytes wrong of 3493: renderer 46, matcher 0 |
| Painting, Full | 46 bytes wrong of 3493: renderer 46, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In `docs/package.json`, two npm scripts gain prettier flags: `"prettier --check ."` ->
`"prettier --cache --check ."` (line 7) and `"prettier --write ."` ->
`"prettier --cache --write --list-different ."` (line 8). The mapping is perfect (each string is
an Update of itself). The single painting (`Minimal`) is scored under both presets, and both
presets give the same runs, all present with the human mapping too: this is how omnidiff narrows
an updated string.

## Painting

46 bytes in each preset, `renderer`:

- Line 7, 8 bytes: the painting inserts ` --cache` after `prettier`. omnidiff's in-string diff
  inserts the same eight characters four places later (`prettier --c` + `ache --c` + `heck .`),
  so ` --c` at columns 23-26 is None -> Insert and ` --c` at columns 31-34 is Insert -> None. Both
  are minimal character edits; only the painting's falls on word boundaries.
- Line 8, 38 bytes: the painting inserts ` --cache` and ` --list-different` and leaves
  `--write` alone. omnidiff aligns the old `write` with other text: it paints `write` on the
  before side as Update (5 bytes), and on the after side the 33 bytes
  ` --cache --write --list-different` disagree with the painting (unpainted at the run's start,
  where the painting has Insert, and painted where the painting leaves `--write`).

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: string Update narrowing in `diff::text`: align on word / flag boundaries
  (whitespace-separated tokens) before characters, so inserted flags come out as whole-word
  inserts.
- Ground truth questionable: no
