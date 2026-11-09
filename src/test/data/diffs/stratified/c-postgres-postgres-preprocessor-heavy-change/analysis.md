# Accuracy analysis: c-postgres-postgres-preprocessor-heavy-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 488 nodes, 488 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 1932: renderer 4, matcher 0 |
| Painting, Full | 10 bytes wrong of 1932: renderer 10, matcher 0 |
| Ground-truth invariant violations | none |

## The change

postgres `contrib/ltree/crc32.c`: the `TOLOWER` macro block (lines 13-18) is removed and
`ltree_crc32_sz` is split by `#ifdef LOWER_NODE`. A new locale-aware version that folds one code
point at a time (after lines 19-49) is inserted. The old function becomes the `#else` branch
(after lines 53-68), minus its `TOLOWER` line, with `&c` replaced by `p`.

## Painting

The mapping is perfect and every run is identical with the human mapping: all renderer.

- Minimal, `None -> Insert`, 4 bytes, the lone `{` and `}` of the newly inserted function and its
  `while` body (after lines 23, 33, 46, 49): the painting paints them as part of the inserted
  function. omnidiff's Minimal preset (`structural_punctuation: false`) drops a range that is only
  structural punctuation in `ranges_for_options` (`src/diff/text/render_options.rs`). It drops these
  lone brackets even when the whole construct they delimit is inserted. `renderer`.
- Full, `Move -> None`, 10 bytes, `, 1);` of `COMP_TRADITIONAL_CRC32(crc, &c, 1);` (before line 34)
  / `COMP_TRADITIONAL_CRC32(crc, p, 1);` (after line 62): `&c` -> `p` is one character shorter, so
  the identical tail shifts one column left, and the Full renderer paints it Move. The painting
  leaves an identical line tail unpainted (painting rule 3). `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter (`ranges_for_options`,
  `src/diff/text/render_options.rs`) for brackets of a wholly inserted construct; the Full
  displaced-tail rule in `identical_or_move`
- Ground truth questionable: no
