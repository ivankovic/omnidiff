# Accuracy analysis: rust-next-font-imports-generator

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 4761 nodes, 4761 graded, N:M floor 0 visible |
| Painting, Minimal | 546 bytes wrong of 21155: renderer 546, matcher 0 |
| Painting, Full | 1459 bytes wrong of 21155: renderer 1459, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Handmade from next.js's font-imports generator (no README). Nested `if let` blocks are collapsed
into `if let ... && let ... {` chains, which reindents their bodies one or two levels (e.g.
before lines 22-24, 98, 160-161 / after lines 22-25, 96, 158-160), one `push(...)` call is
rewrapped by rustfmt (before lines 166-178, after lines 165-180), and the items of two `use` lists
are reordered (`{errors::HANDLER, Spanned, DUMMY_SP}` -> `{DUMMY_SP, Spanned, errors::HANDLER}` on
line 4, `{noop_visit_type, Visit}` -> `{Visit, noop_visit_type}` on line 7).

## Painting

The mapping matches the ground truth on every node, and both presets render identically with the
omnidiff and the human mapping, so every byte is the renderer's (or the painting's).

Minimal, 546 bytes:

- `None -> Move`, 384 bytes: the `let` conditions that move from their own nested `if` into the
  chain (`let Expr::Ident(ident) = &**callee_expr`, `let Some(font_function) = ...`, `let
  Expr::Call(call_expr) = &**expr`, `let Some(ident) = self.check_var_decl(var_decl)`, both
  sides). omnidiff leaves them unpainted with either mapping and under both presets. Where a
  condition was on the second level, its `let` keeps row and column (`if ` becomes `&& ` in
  place) and there is no displacement to see; for deeper levels the column changes, and which
  exemption in `identical_or_move` keeps them unpainted was not traced. The painting marks all of
  them moved into the chain. `renderer`.
- `Move -> None`, 104 bytes: `self.state.font_exports.push(ModuleItem::ModuleDecl(` and
  `ModuleDecl::ExportNamed(` (before lines 166-167), which rustfmt rewrapped onto new lines;
  omnidiff paints them `Move`, the painting leaves the rewrap alone. `renderer`.
- `None -> Move`, 46 bytes: the swapped `errors::HANDLER` / `DUMMY_SP` on line 4. On a row with
  the same index, `node_untouched_on_its_row` accepts a node placed uniquely on an edited row as
  untouched, so the swap reads as a displacement and is not painted. `renderer`.
- About 12 bytes of lone `{`, `}`, `),`, `,`, `)` that the painting deletes, moves or inserts and
  Minimal drops as structural punctuation. `renderer`.

Full, 1459 bytes:

- About 950 bytes of indentation on the reindented bodies: the Full painting paints the removed
  indentation `Delete` and the rest `Move`, while omnidiff's `Move` covers the whole leading
  whitespace (`Move -> Delete` 669 bytes, plus `None`/`Delete -> Move`/`Insert` runs). `renderer`.
- `None -> Move`, 445 bytes: the same chained `let` conditions as in Minimal, plus the first
  condition line `if let Callee::Expr(callee_expr) = &call_expr.callee`. `renderer`.
- `Move -> None`, 22 bytes: `, Spanned, ` on line 4, displaced by the swap and painted `Move`
  under Full's `paint_displaced_moves`. `renderer`.
- `Move -> None`, 42 bytes: the swap `noop_visit_type` / `Visit` on line 7. omnidiff paints it
  `Move` (as it paints the line 4 swap); neither painting marks it, although both mark the same
  kind of swap on line 4. `ground-truth`.

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth`
- Where a fix would go: `src/diff/text.rs` `identical_or_move` (conditions joining a let chain
  keep their position; a uniquely placed item swapped within its row is not "untouched"; rewrapped
  call heads under Minimal) and Full's indentation split for reindented bodies
- Ground truth questionable: yes: the `visit::{noop_visit_type, Visit}` swap on line 7 is left
  unpainted in both paintings, while the same kind of swap on line 4 is painted `Move`
