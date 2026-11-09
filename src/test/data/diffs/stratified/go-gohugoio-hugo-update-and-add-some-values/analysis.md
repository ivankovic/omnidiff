# Accuracy analysis: go-gohugoio-hugo-update-and-add-some-values

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 139 nodes, 139 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 1740: renderer 0, matcher 0 |
| Painting, Full | 4 bytes wrong of 1740: renderer 4, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Hugo's `common/hugo/version_current.go` version bump: `Minor: 154` becomes `155` (line 22),
`PatchLevel: 2` becomes `0` (line 23) and `Suffix: ""` becomes `"-DEV"` (line 24).

## Painting

Minimal matches exactly. The Minimal painting updates only the changed last digit of the minor
version. In Full, the 4 bytes are the same with the human mapping, so they are the renderer's.

- Full, `None -> Update`, line 22, 2 bytes per side: the unchanged leading digits of
  `154` -> `155`. Omnidiff's Full rendering narrows the number literal's Update to its last
  digit, exactly as in Minimal. The Full painting updates the whole literal, treating a number as
  one token. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Update narrowing in the renderer. Under Full, a number literal should
  be painted whole.
- Ground truth questionable: no
