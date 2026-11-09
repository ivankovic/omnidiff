# Accuracy analysis: c-awesomewm-awesome-add-function

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 6307 nodes, 6307 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 39008: renderer 2, matcher 0 |
| Painting, Full | 0 bytes wrong of 39008: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

awesome `root.c`: a new function `luaA_root_get_content` (after lines 504-525) and its entry
`{ "content", luaA_root_get_content},` in the method table (after line 636). Insertions only.

## Painting

- **Minimal, None -> Insert, 2 bytes:** the `{` and `}` that open and close the new function's
  body, each alone on its line (after lines 511 and 525). The Minimal painting includes them in
  the inserted function; omnidiff's Minimal rendering drops lone brackets as punctuation. The
  same with the human mapping. `renderer`.

Full agrees with the painting; the mapping has no mismatches.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` Minimal punctuation filter: keep brackets that belong to a
  wholly inserted construct
- Ground truth questionable: no
