# Accuracy analysis: python-portagefilelist-client-remove-one-import-and-update-one-const-string

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 8 (4 visible) of 3380 nodes, 3380 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`pfl/e_file.py` imports `portage` twice: line 4 in the standard-library group, and line 9 under
`# portage api: sys-apps/portage`. The commit removes the line-4 copy and bumps `VERSION='3.4'` to
`'3.5'`.

## Mapping

- **Wrong twin of `import portage` kept (4 visible, 4 invisible), `matcher`.** The ground truth
  deletes before line 4 and keeps before line 9 as after line 8, in place, under its comment. This
  agrees with a line diff. omnidiff pairs before line 4 with after line 8 (`IdenticalHash`, its
  children `IdenticalHashOfAncestor`: 2 visible, 2 invisible) and deletes before line 9
  (`APTED("fast_fallback")`: 2 visible, 2 invisible). The pair comes from the kind-and-value tier of
  the phase-1 hash descent (`solve_hash_descent` -> `hash_tree_matching::solve_with_hash_map`). It
  visits before nodes largest first with ties by start byte, so line 4 is visited before line 9 and
  takes the only after copy; start-byte proximity is used only to choose among after candidates,
  never among before ones. The two statements are byte-identical, but omnidiff's choice moves the
  import across `from datetime import datetime` and two comments. A reader sees a moved line and a
  deletion where there is only one deletion. Picking the twin that keeps sibling order (no crossing)
  is better here.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: twin selection in `hash_tree_matching::solve_with_hash_map`
  (`IdenticalHash`) when two before copies compete for one after copy: prefer the candidate that
  preserves order relative to matched neighbours, not the first in byte order.
- Ground truth questionable: no
