# Accuracy analysis: c-freeciv-freeciv-change-define-but-also-keep-the-old-working-with-a-move

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 7 (5 visible) of 2224 nodes, 2224 graded, N:M floor 0 visible |
| Painting, Minimal | 90 bytes wrong of 23529: renderer 30, matcher 78 |
| Painting, Full | 91 bytes wrong of 23529: renderer 30, matcher 90 |
| Ground-truth invariant violations | none |

## The change

freeciv `common/tech.h`: `#define SPECENUM_VALUE12 TECH_USER_LAST` (line 96) becomes
`#define SPECENUM_VALUE12 TECH_USER_10`, and a new `#define TECH_USER_LAST TECH_USER_10` (after
line 103) keeps the old name working. The single painting (`Only one solution`) records
`TECH_USER_LAST` as moved from the value of line 96 to the name of the new define.

## Mapping

- **Line 96's define paired with the new define, 5 visible + 2 invisible.** Ground truth: line 96
  stays line 96 (`#define`, `SPECENUM_VALUE12` identical, value replaced), its `TECH_USER_LAST`
  moves to the name of the new define (`preproc_arg` -> `identifier`), and the new define's
  `#define` and `TECH_USER_10` are inserted. omnidiff maps before line 96 onto after line 103
  (`SPECENUM_VALUE12` -> `TECH_USER_LAST`, `TECH_USER_LAST` -> `TECH_USER_10`, both Update) and
  inserts after line 96 whole (`APTED("large_flat_subtree")`, the Myers fast path over the
  header's flat list of top-level items). Keeping the name `SPECENUM_VALUE12` would have cost one
  Update instead of two; why the residual pairing picked the later define is undetermined from
  the evidence. `matcher`.

## Painting

Both presets are scored against the one painting.

- **Matcher, 76 bytes per preset:** `SPECENUM_VALUE12` painted Update on before line 96 (16, Full
  31 with the value), `#define SPECENUM_VALUE12 ` painted Insert on after line 96 (25), and the
  new `#define TECH_USER_LAST TECH_USER_10` left unpainted (35) because omnidiff maps it to the
  old line. `matcher`.
- **Renderer, 14 + 16 bytes (also with the human mapping):** the painting's Move of
  `TECH_USER_LAST` from a `preproc_arg` (before line 96) to an `identifier` (after line 103) is
  rendered as nothing on both sides: the renderer does not paint this cross-kind pair Move. The
  after-side run starts at the space before the name, inside the painting's `#define ` insert.
  `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: the residual pairing in `solve_large_flat_subtrees` / the Myers fast path
  of `apted::common::resolve` (prefer the same-name `preproc_def`); cross-kind Move rendering in
  `diff::text`
- Ground truth questionable: no
