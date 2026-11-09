# Accuracy analysis: java-nextcloud-android-add-if-branch-with-reused-return

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 54 (38 visible) of 35927 nodes, 35927 graded, N:M floor 0 visible |
| Painting | none on file |
| Ground-truth invariant violations | none |

## The change

In Nextcloud Android's `OCFileListFragment`, the encryption-setup result handler stops failing at
once when `mAdapter.getItem(position)` returns null. It now logs a warning (first `if (file ==
null)`, line 1281), retries the lookup by remote path (second, line 1285), and only then logs the
error and returns (third, lines 1290-1293). A related call passes `file.getRemotePath()` instead of
`position` (line 1141). No painting on file.

## Mapping

All 54 mismatches (38 visible) have reason `APTED("large_flat_subtree")`
(`solve_large_flat_subtrees`, which handles this very large method).

- **Which null check the old one became (38 visible, 16 invisible), `ambiguous`.** The old block
  (lines 1281-1284) is `if (file == null) { Log_OC.e(TAG, "file is null cannot toggle encryption");
  return; }`.
  - The ground truth maps the old `if`, its condition and its log statement to the first new check,
    `if (file == null) { Log_OC.w(TAG, "file is null, checking via remote path"); }` (lines
    1281-1283). It maps `e` -> `w` and the message as Updates. It maps the old `return;` into the
    third check (line 1292), and inserts the third check's `if`, braces and `Log_OC.e(TAG,"file is
    null, cannot toggle encryption");` as new.
  - omnidiff maps the old block whole to the third check (lines 1290-1293). The `if` header and `}`
    are Identical. `Log_OC.e(TAG, ...)` maps to `Log_OC.e(TAG, ...)`, with the message an Update
    (one comma and one space added). The first check is inserted.

  Both are one-to-one and both keep the `return`. The ground truth reads the edit as "the old check
  was kept and demoted to a warning, and a new final check reuses its return". This matches the
  fixture's name. omnidiff reads it as "the old check moved to the end, unchanged except for
  punctuation in its message", which is the closer textual match. Neither mapping is wrong on its
  own terms.

## Verdict

- Primary cause: `ambiguous`
- Other causes: `ground-truth`
- Where a fix would go: none: ambiguous; if the ground truth's reading is the intended one, a pass
  would have to prefer the first structurally identical `if (file == null)` over the textually
  closest one, which is not a general rule
- Ground truth questionable: yes, mildly: the third check's `Log_OC.e(TAG,"file is null, cannot
  toggle encryption")` is recorded as inserted although it differs from the old log line only by a
  comma and a space; the fixture name suggests this is deliberate (Marko's call)
