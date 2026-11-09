# Accuracy analysis: go-jesseduffield-lazygit-nice-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 12 (7 visible) of 26959 nodes, 26959 graded, N:M floor 0 visible |
| Painting, Minimal | 27 bytes wrong of 173298: renderer 16, matcher 26 |
| Painting, Full | 31 bytes wrong of 173298: renderer 20, matcher 29 |
| Ground-truth invariant violations | none |

## The change

In a vendored Windows syscall file, a `//sys WSADuplicateSocket` directive is added (after line
873). In `NTUnicodeString.Slice` (before lines 1701-1702),
`slice := unsafe.Slice(s.Buffer, s.MaximumLength)` / `return slice[:s.Length]` becomes a comment
plus `return unsafe.Slice(s.Buffer, s.MaximumLength/2)[:s.Length/2]` (after lines 1702-1704).
The local variable is inlined.

## Mapping

All 12 mismatches (7 visible) are on before lines 1701-1702, from `APTED("qualified_name")`. The
ground truth keeps the `return_statement`, its `slice_expression` (`[`, `:`, `]`), and the
`unsafe.Slice(...)` call with its `(`, `,`, `)`. The call moves from the deleted
`short_var_declaration` into the slice operand. Omnidiff deletes `return`, `[`, `:`, `]` and the
call's `(`, `,`, `)`, together with their parent nodes. The call's identifiers and arguments are
not in the mismatch list, so they are matched. APTED has no move: it cannot keep the return
statement and also re-parent the call from the deleted declaration into it, so it gave up the
glue of one of the two. `matcher`.

## Painting

Both paintings move `return`, the call, `)`, `[:s.Length` and `]`, insert the two `/2`, and delete
`slice :=` and `slice`.

- Matcher, about 11 bytes in each preset: `return`, `[:`, `]`, `(`, `, ` and `)` are painted
  Delete/Insert by omnidiff (or unpainted in Minimal) where the painting moves them.
- Renderer, Minimal 16 bytes, Full 20 bytes, `None -> Move` (Full also `None -> Delete`). These
  runs are also present with the human mapping. `return` and the `)` and `]` shifted to new rows
  and columns, and the painting marks them Move. Omnidiff leaves them unpainted: Minimal drops
  the lone brackets, and `return` counts as unchanged in place. In Full, the deleted line's
  indentation tab and a space are also missed. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: the APTED call at `qualified_name`
  (`solve_syntax_aware_matching.rs`). It needs a move-aware step for an inlined variable, where a
  declaration whose value moves into its only use keeps the use's statement. Rendering of shifted
  `return` and brackets is a separate, smaller issue.
- Ground truth questionable: no
