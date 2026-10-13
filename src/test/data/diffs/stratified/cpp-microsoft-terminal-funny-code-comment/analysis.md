# Accuracy analysis: cpp-microsoft-terminal-funny-code-comment

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 2929 nodes, 2929 graded, N:M floor 0 visible |
| Painting, Minimal | 84 bytes wrong of 14328: renderer 84, matcher 0 |
| Painting, Full | 8 bytes wrong of 14328: renderer 8, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`TitlebarControl::CaptionButtonWidth()` returns `float` instead of `double`. The cached
`static auto width{ MinMaxCloseControl().ActualWidth() / 3.0 };` becomes
`const auto minMaxCloseWidth = MinMaxCloseControl().ActualWidth();`, and the division moves into the
`return` (lines 51-52). description.md: "One of the funniest pieces of code."

## Painting

All runs are the same with the human mapping.

- **Kept tokens painted Move (Minimal, 76 bytes, `Move -> None`, `renderer`).** In line 51, `static`
  becomes `const`, the brace initializer becomes `= ...`, and `/ 3.0` leaves. Omnidiff paints the
  surviving `auto` and `MinMaxCloseControl().ActualWidth()` Move on both sides. The Minimal painting
  leaves them unpainted. The Full painting does mark them Move, and Full agrees there.
- **Rename split inside a word (Minimal, 8 bytes, `None -> Update`, `renderer`).** For `width` ->
  `minMaxCloseWidth`, omnidiff narrows the Update and leaves the shared suffix `idth` unpainted on
  both sides. The painting paints the whole identifier. This is the "shared suffix inside a word"
  case in text_painting_findings.md rule 3.
- **Separating spaces (Full, 8 bytes, `renderer`).** Single spaces between the Move tokens on line
  51 (and one deleted or inserted space) are left unpainted. The Full painting includes them.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs`. Minimal Move display of tokens that stay in a reshaped
  declaration; identifier-update narrowing, which should not split at an in-word suffix; Full gap
  spaces.
- Ground truth questionable: no
