# Accuracy analysis: rust-gyulyvgc-sniffnet-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 1259 nodes, 1259 graded, N:M floor 0 visible |
| Painting, Minimal | 96 bytes wrong of 5092: renderer 81, matcher 15 |
| Painting, Full | 50 bytes wrong of 5092: renderer 31, matcher 19 |
| Ground-truth invariant violations | none |

## The change

From GyulyVGC/sniffnet (`src/gui/styles/toggler.rs`, commit fcc5d5c6): an iced API upgrade.
`use iced::Color;` moves below the toggler import and becomes `use iced::{Background, Color};`;
in two `Style { ... }` literals `background` and `foreground` values are wrapped in
`Background::Color(...)` and three fields are appended; and the match arm `Status::Disabled =>
class.active(self, false)` becomes `Status::Disabled { is_toggled } => class.active(self,
is_toggled)` (before line 82, after line 88). `description.md`: "Contains a false->identifier
mapping which is tricky in the AST."

## Mapping

Two visible mismatches on before line 82, reason `APTED("qualified_name")`. The ground truth
maps the `::` of the pattern `Status::Disabled` (a `scoped_identifier`) to the `::` of the new
struct pattern `Status::Disabled { is_toggled }`, and the argument `false` (a `boolean_literal`)
to `is_toggled` (an `identifier`) as an update. omnidiff deletes both: the pattern changed node
kind, and a `false` -> identifier pairing crosses node kinds, which the tree edit costs do not
favour. `matcher`.

## Painting

Matcher-caused (Minimal 15 bytes, Full 19): `false` / `is_toggled` on line 82/88 painted `Delete`
/ `Insert` instead of `Update`; in Full also the `::` painted `Delete`/`Insert`.

Renderer-caused (present with the human mapping too):

- Minimal 23 bytes, Full 21: `use iced::` and `;` of the reordered import (before line 5, after
  line 6), `None -> Move`: the one-line `use` moved below its sibling, kept its column and only
  changed row, so `identical_or_move` does not paint it moved (the `crossed_backwards` check is for
  multi-row nodes); plus the inserted space in `{Background, Color}` left unpainted.
- Minimal 42 bytes, `Move -> None`, `=> class.active(self,` on before line 82 / after line 88: the
  inserted `{ is_toggled }` pushed it right, and since the row also changed later (`false` ->
  `is_toggled`) the Minimal displacement exemption does not apply, so it is painted `Move`.
- Minimal 16 bytes, `Move -> None`, `bg_color` in `background: bg_color` of the second literal
  (before line 57, after line 60), wrapped into `Background::Color(...)` and painted `Move`. The
  identical edit in the first literal (line 31) is not painted; why the two differ was not
  traced.
- Full, the rest: the `,` after wrapped values painted `Move`, and the spaces before `false` /
  `is_toggled` that the Full painting moves.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`
- Where a fix would go: `src/diff/text.rs` `identical_or_move` (single-row sibling reorder;
  displacement exemption on a row that also changed further right); APTED cost of a
  `boolean_literal` -> `identifier` update and of `scoped_identifier` -> struct pattern
- Ground truth questionable: no
