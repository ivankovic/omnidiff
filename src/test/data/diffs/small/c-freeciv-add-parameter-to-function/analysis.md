# Accuracy analysis: c-freeciv-add-parameter-to-function

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 254 nodes, 254 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 2579: renderer 0, matcher 0 |
| Painting, Full | 29 bytes wrong of 2579: renderer 29, matcher 0 |
| Ground-truth invariant violations | none |

## The change

freeciv `ai/default/daiair.h`: the prototype of `dai_choose_attacker_air` gets a new parameter
`const struct civ_map *nmap,` before `struct player *pplayer,`, which wraps onto a new
continuation line (after line 25).

## Painting

- **Full, None -> Insert, 29 bytes:** the alignment whitespace at the start of the new
  continuation line (after line 25). The Full painting paints it Insert; omnidiff leaves leading
  whitespace unpainted. The same with the human mapping. `renderer`.

Minimal agrees with the painting; the mapping has no mismatches.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` Full rendering of the indentation of a line created by an
  insert-and-rewrap
- Ground truth questionable: no
