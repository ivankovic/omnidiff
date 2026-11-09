# Accuracy analysis: kotlin-nextcloud-remove-function

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 5 (0 visible) of 14174 nodes, 14174 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`BackgroundJobManagerImpl.kt` removes `schedulePeriodicFilesSyncJob` (before lines 480-512),
renames `startAutoUploadImmediately` to `startAutoUpload`, and inserts
`.setExpedited(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST)` into the builder chain
`oneTimeRequestBuilder(...).setInputData(arguments).setConstraints(...)...` (after line 503).
The content-observer job also switches from one-time to periodic work.

## Mapping

All five mismatches are invisible wrapper nodes of the builder chain at before line 531 / after
line 498. The Kotlin chain is left-nested (`call_expression` / `navigation_expression` pairs), so
inserting `.setExpedited(...)` in the middle adds one wrapper pair.

- Omnidiff maps the sub-chain below the insertion to its identical twin:
  `oneTimeRequestBuilder(...)`, `oneTimeRequestBuilder(...).setInputData` and
  `....setInputData(arguments)` before -> the same text after (Identical; reasons `WrapGrowth`
  and `APTED("large_flat_subtree")`).
- The ground truth pairs the wrappers by depth from the top of the chain
  (`match_but_not_identical` down to the depth of `oneTimeRequestBuilder(...)`), and marks the
  two innermost after wrappers (`oneTimeRequestBuilder(...)` and
  `oneTimeRequestBuilder(...).setInputData`) as inserted. Its leaf pairs are the other way: the
  `oneTimeRequestBuilder` identifier and its argument list map Identical to the children of the
  after `oneTimeRequestBuilder(...)` call that the ground truth itself marks inserted, and the
  `.setInputData` tokens map into the after wrapper it marks inserted. So the ground truth's
  wrapper pairs disagree with its own leaf pairs (a before parent mapped to one after node, its
  children to the children of a different, "new", after node). `invariants.txt` does not catch
  this.

Nothing a reader sees differs. `ground-truth`, and invisible-only (`grading-artifact`).

## Verdict

- Primary cause: `ground-truth`
- Other causes: `grading-artifact`
- Where a fix would go: ground truth (Marko's call); omnidiff's mapping is consistent with the
  ground truth's own leaf pairs.
- Ground truth questionable: yes: in the builder chain at before line 531 the wrapper nodes are
  paired by depth while their leaves are paired with the children of after wrappers marked
  inserted.
