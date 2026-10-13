# Accuracy analysis: c-linux-small-bugfix

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 34 (24 visible) of 47986 nodes, 47994 graded, N:M floor 6 visible |
| Painting, Minimal | 143 bytes wrong of 199613: renderer 127, matcher 44 |
| Painting, Full | 45 bytes wrong of 199613: renderer 16, matcher 47 |
| Ground-truth invariant violations | none |

## The change

linux `arch/x86/events/intel/ds.c`: in one function, `val |= omr.omr_hitm ? P(SNOOP, HITM) :
P(SNOOP, HIT);` (before line 348) moves from before the `if (omr.omr_source == 0x2)` block into
its `else if` branch (after line 362); the `== 0x2` branch gets a new first case
`if (omr.omr_hitm) val |= P(SNOOP, HITM);` (after lines 351-352); `omr_promoted` is shifted
(`<< 1`); and a new `else { val |= P(SNOOP, NONE); }` is added (after lines 364-366).

## Mapping

- **New `val |= P(SNOOP, NONE);` in the added `else` (after line 365), 18 visible + 8
  invisible.** Ground truth: inserted. omnidiff maps the unchanged `val |= P(SNOOP, NONE);` of
  the `snoop == 0x3` case (before line 360 -> after line 360) and also to the new copy
  (`MultiMap`, `solve_multi_maps`), so each of its nodes "mapped to X and 1 more" and the new
  statement is not an insert. The original stayed where it was and the new line is new code; a
  copy reading is defensible only loosely. `matcher`.
- **`P(SNOOP, HITM)` in the new `if (omr.omr_hitm)` (after line 352), 6 visible + 2 invisible.**
  Ground truth: a 1:2 all-to-all group with the `P(SNOOP, HITM)` of the moved line. omnidiff
  inserts it (`APTED("qualified_name")`). This is the N:M floor (6 visible). `nm-floor`.

## Painting

- **Minimal, None -> Move, 106 bytes (also with the human mapping):** the moved line on before
  line 348 and after line 362. The painting marks it Move; omnidiff's Minimal rendering leaves it
  unpainted on both sides, while its Full rendering paints it Move. The line moved into a nested
  branch and gained a tab of indentation; which Minimal rule suppresses the Move is undetermined
  (checked: the mapping pairs it, and Full paints it). `renderer`.
- **Minimal 22 / Full 24 bytes, None -> Insert:** the new `val |= P(SNOOP, NONE);` (after line
  365) left unpainted because of the `MultiMap` copy above. `matcher`.
- **Minimal 15 / Full 14 bytes, Insert -> Move:** `P(SNOOP, HITM)` on after line 352, the second
  destination of the painting's 1:2 Move. `nm-floor`.
- **Full, 7 bytes (also with the human mapping):** single tabs, spaces and `;` around the edited
  lines (before 348, 351; after 349, 362, 364). `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`, `nm-floor`
- Where a fix would go: `diff::text` Minimal Move painting of a line moved into a nested branch;
  `solve_multi_maps` should not add a copy when the original is unchanged in place
- Ground truth questionable: no
