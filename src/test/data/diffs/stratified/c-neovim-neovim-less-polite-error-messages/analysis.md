# Accuracy analysis: c-neovim-neovim-less-polite-error-messages

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 54062 nodes, 54062 graded, N:M floor 0 visible |
| Painting, Minimal | 14 bytes wrong of 248374: renderer 14, matcher 0 |
| Painting, Full | 22 bytes wrong of 248374: renderer 22, matcher 0 |
| Ground-truth invariant violations | none |

## The change

neovim `src/nvim/spellsuggest.c`: two messages lose their "Sorry, ": `"Sorry, no suggestions"`
-> `"No suggestions"` (line 517) and `"Sorry, only %" ...` -> `"Only %" ...` (line 520).

## Painting

The mapping has no mismatches; all runs are the same with the human mapping.

- **Update -> Delete, 14 bytes, both presets:** `Sorry, ` on before lines 517 and 520. omnidiff
  renders the string change as an Update narrowed to the removed prefix; both paintings delete
  it. `renderer`.
- **Full, None -> Update, 8 bytes:** `o` of `no` / `No` and `nly` of `only` / `Only`, both sides.
  The Full painting updates the whole word whose case changed; omnidiff narrows the Update to the
  single changed letter. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` Update narrowing inside string literals: a removed prefix
  as Delete, and whole-word Updates under Full
- Ground truth questionable: no
