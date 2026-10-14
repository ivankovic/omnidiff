# Accuracy analysis: tsx-shadcn-ui-ui-move-class-around-in-string

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 342 nodes, 342 graded, N:M floor 0 visible |
| Painting, Minimal | 300 bytes wrong of 1942: renderer 300, matcher 0 |
| Painting, Full | 300 bytes wrong of 1942: renderer 300, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Two Tailwind `className` strings are reordered: line 8 moves `bg-muted` from the front to before
`p-6`, line 11 moves `bg-primary text-primary-foreground` from the front to the end.

## Painting

All runs are identical with the human mapping, so none is the matcher's.

- 300 bytes per preset (66 bytes per side on line 8, 84 per side on line 11),
  before Update -> Move and after Update -> None (`renderer`). Each class list is one
  `string_fragment` leaf, mapped as an Update. omnidiff narrows it to the span between the common
  prefix and suffix, which covers almost the whole list, and paints it Update. The painting
  instead marks only the moved class as Move (`bg-muted`, `bg-primary`) and leaves the other
  classes unpainted. A word-level move inside a string leaf is not something the renderer does.
- Ground-truth note, line 11: both `bg-primary` and `text-primary-foreground` moved (the
  unchanged order is `flex size-6 items-center justify-center rounded-md`), but the painting
  moves only `bg-primary`; `text-primary-foreground` (23 bytes per side) is left unpainted.
  Those bytes fall inside omnidiff's Update runs and count as wrong either way.

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth`
- Where a fix would go: src/diff/text.rs, Update narrowing of a string leaf (token-level diff
  with moves inside whitespace-separated class lists)
- Ground truth questionable: yes: on line 11 the painting moves `bg-primary` but leaves the
  also-moved `text-primary-foreground` unpainted
