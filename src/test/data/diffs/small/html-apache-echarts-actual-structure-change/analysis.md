# Accuracy analysis: html-apache-echarts-actual-structure-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 13 (9 visible) of 7697 nodes, 7697 graded, N:M floor 0 visible |
| Painting | no painting |
| Ground-truth invariant violations | none |

## The change

In the test-runner page's run-config block, the "Expected" and "Actual" version pickers are
reworked the same way: the "Use Nightly Build" tooltip/checkbox inside each `<span class="label">`
is removed, a new source `<el-select>` with four `<el-option>`s is added, the version
`<el-select>` gets a `v-if` and its attributes one per line, and an `<el-input>` for PR numbers
is added. A commented-out `<!-- <i class="el-icon-link"></i> -->` is replaced as well.

## Mapping

- "Actual" half left to the fallback (9 visible, 4 invisible; `matcher`, reason
  `APTED("fast_fallback")`, all Delete). The ground truth maps the before `<span class="label">`
  holding `Actual` (line 157) to the after one (line 172), its `Actual` text (line 158) identical,
  and the before version `<el-select ...>` start tag (lines 163-165) to the after version select's
  start tag (lines 180-187), keeping `<`, `el-select`, `>` and the `:style` attribute (its value
  updated from `isActualNightly ? 160 : 80` to `actualSource === 'nightly' ? 160 : 80`). omnidiff
  deletes all of these. The mirrored "Expected" half (lines 145-153 before) has no mismatches,
  so the same shapes are resolved there by earlier passes; the Actual half reaches the final
  Myers-LCS fallback, which leaves them unpaired. Why the earlier passes stop short on the second
  half is undetermined from this evidence (the fixture's test file says "Residual not yet
  examined").

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: whichever pass resolves the Expected half (src/diff.rs pass order before
  `apted::for_roots_fallback`) - it should also pair the Actual span and version select
- Ground truth questionable: no
