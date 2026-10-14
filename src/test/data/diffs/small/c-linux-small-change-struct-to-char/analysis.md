# Accuracy analysis: c-linux-small-change-struct-to-char

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 22754 nodes, 22754 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 86628: renderer 0, matcher 0 |
| Painting, Full | 186 bytes wrong of 86628: renderer 186, matcher 0 |
| Ground-truth invariant violations | none |

## The change

linux `tools/objtool/klp-diff.c`: `struct symbol *code_sym` becomes `const char *code_sym`
(line 1355), it is assigned `reloc->sym->name` instead of `reloc->sym` (line 1375), a
`"<unknown>"` fallback is inserted (after lines 1398-1400), and its four uses `code_sym->name`
become `code_sym` (before lines 1401, 1408, 1414, 1425).

## Painting

The mapping has no mismatches; Minimal agrees with the painting. All runs are the same with the
human mapping.

- **Full, Move -> None, 186 bytes:** the unchanged line tails after the four shortened arguments
  (`, reloc->sym->name + 13);`, `);`, `, code_offset, reloc->sym->name);`, on both sides). Deleting
  `->name` shifts them left and the inserted lines shift them down three rows; Full paints them
  Move, the painting leaves an identical line tail alone. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` Full displacement exemption for tails shifted in both row
  and column (`displaced_beside_an_edit_on_its_first_row` / `shifted_by_an_edit_beside_it`)
- Ground truth questionable: no
