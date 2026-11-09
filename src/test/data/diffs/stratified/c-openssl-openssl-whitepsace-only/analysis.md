# Accuracy analysis: c-openssl-openssl-whitepsace-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 142 nodes, 142 graded, N:M floor 0 visible |
| Painting, Minimal | 297 bytes wrong of 1407: renderer 297, matcher 0 |
| Painting, Full | 297 bytes wrong of 1407: renderer 297, matcher 0 |
| Ground-truth invariant violations | none |

One painting (`Only one solution`), scored under both presets.

## The change

openssl `crypto/ui/ui_null.c`, whitespace only (`description.md`: "Whitespace only"): in the
initializer on lines 14-18 the padding between `NULL,` and each trailing comment (`/* opener */`
etc.) shrinks from 24 spaces to one. The commit is the same reformat commit (`2fab90bb5`) as
`c-openssl-openssl-format-only-change`.

## Painting

The mapping is perfect and the runs are identical with the human mapping.

- Both presets, `None -> Update`, lines 14-18 on both sides, 297 bytes (206 before, 91 after): the
  painting has one Match per line, from `NULL,` to the end of the comment, and since the two texts
  differ (in whitespace only) each renders as Update. omnidiff paints nothing: `NULL`, `,` and the
  comment are identical tokens, and the comment's column shift is a sideways shift on its own row
  index, which `identical_or_move` (`src/diff/text.rs`) does not count as a relocation.
- Under Minimal this conflicts with the convention of the sibling fixture from the same commit:
  `c-openssl-openssl-format-only-change`'s Minimal painting is empty ("Whitespace only. Minimal
  painting is empty."), so the same whitespace-only realignment is painted there and not here.
  `ground-truth`.
- Under Full the sibling fixture paints realigned text as Move rather than Update, so this
  painting's label also differs from it. But omnidiff's Full rendering paints nothing at all,
  which would disagree with either label. `renderer`.

## Verdict

- Primary cause: `ground-truth`
- Other causes: `renderer`
- Where a fix would go: ground truth (Marko's call) for the Minimal reading; for Full, the
  treatment of whitespace-only realignment of unchanged tokens in `identical_or_move`
  (`src/diff/text.rs`)
- Ground truth questionable: yes: the single painting paints a whitespace-only change as five
  Updates. The sibling fixture from the same reformat commit leaves its Minimal painting empty and
  paints the realigned text Move in Full.
